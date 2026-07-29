use aviutl2::{anyhow::Context, tracing};

use crate::evaluate_chars::{
    char_states_to_text, controls_between_to_text, controls_to_text, evaluate_chars,
};
use crate::lua_handle::{FullTextDecoration, LuaHandle, TextLayout};
use crate::segment;

#[derive(Debug, serde::Serialize)]
pub struct Layout {
    content: String,
    position: (f64, f64),
    size: (f64, f64),
}

fn apply_negative_center_delta(x: f64, y: f64, text_layout: TextLayout) -> (f64, f64) {
    (
        x - text_layout.negative_center_x_delta,
        y - text_layout.negative_center_y_delta,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Justify {
    No,
    SpecifiedWidth,
    LongestLine,
}
impl<'a> aviutl2::module::FromScriptModuleParamTable<'a> for Justify {
    type Error = aviutl2::module::ParamConversionError;
    fn from_param_table(
        param: &'a aviutl2::module::ScriptModuleParamTable,
        key: &str,
    ) -> Result<Self, aviutl2::module::GetParamError<Self::Error>> {
        let value = param.get_int(key);
        match value {
            0 => Ok(Self::No),
            1 => Ok(Self::SpecifiedWidth),
            2 => Ok(Self::LongestLine),
            _ => Err(aviutl2::module::GetParamError::ConversionError(
                aviutl2::module::ParamConversionError::new(format!(
                    "Invalid value for Justify: {}. Expected 0, 1, or 2.",
                    value
                )),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlign {
    Left,
    Center,
    Right,
    Justify,
}

impl<'a> aviutl2::module::FromScriptModuleParamTable<'a> for HorizontalAlign {
    type Error = aviutl2::module::ParamConversionError;
    fn from_param_table(
        param: &'a aviutl2::module::ScriptModuleParamTable,
        key: &str,
    ) -> Result<Self, aviutl2::module::GetParamError<aviutl2::module::ParamConversionError>> {
        let value = param.get_int(key);
        match value {
            0 => Ok(Self::Left),
            1 => Ok(Self::Center),
            2 => Ok(Self::Right),
            3 => Ok(Self::Justify),
            _ => Err(aviutl2::module::GetParamError::ConversionError(
                aviutl2::module::ParamConversionError::new(format!(
                    "Invalid value for Justify: {}. Expected 0, 1, or 2.",
                    value
                )),
            )),
        }
    }
}

#[derive(aviutl2::module::FromScriptModuleParam)]
pub struct LayoutParams {
    pub lua_callback: String,
    pub width: usize,
    pub align: HorizontalAlign,
    pub justify: Justify,
    pub segmentation_mode: segment::SegmentationMode,
    pub text: String,
    pub size: f64,
    pub line_spacing: f64,
    pub char_spacing: f64,
    pub show_speed: f64,
    pub font: String,
    pub color: u32,
    pub secondary_color: u32,
    pub outline_size: f64,
    pub decoration: FullTextDecoration,
    pub bold: bool,
    pub italic: bool,
    pub time: f64,
}

#[derive(Debug)]
struct WrappedLine {
    chars: Vec<crate::evaluate_chars::CharState>,
    control_index: usize,
    is_paragraph_end: bool,
}

#[derive(Debug)]
struct SourceLine {
    chars: Vec<crate::evaluate_chars::CharState>,
    control_index: usize,
}

/// Returns wrapped lines paired with a flag indicating whether the line is the last
/// in its explicit paragraph (i.e., followed by `\n` or end-of-input). Justify should
/// not be applied to paragraph-ending lines.
fn build_wrapped_lines(
    lines: &[SourceLine],
    controls: &[aviutl2_text_parser::Element],
    lua_handle: &LuaHandle,
    decoration: FullTextDecoration,
    char_spacing: f64,
    width: usize,
    segmentation_mode: segment::SegmentationMode,
) -> aviutl2::AnyResult<Vec<WrappedLine>> {
    let mut wrapped_lines: Vec<WrappedLine> = Vec::new();
    for SourceLine {
        chars: line_chars,
        control_index,
    } in lines
    {
        if line_chars.is_empty() {
            wrapped_lines.push(WrappedLine {
                chars: vec![],
                control_index: *control_index,
                is_paragraph_end: true,
            });
            continue;
        }
        let mut segmented = segment::segment(line_chars, segmentation_mode)
            .into_iter()
            .collect::<std::collections::VecDeque<_>>();
        let mut current_line = vec![];
        tracing::trace!("Processing line: {line_chars:#?}");
        while let Some(segment) = segmented.pop_front() {
            'try_push: loop {
                let segment_text = char_states_to_text(
                    &current_line
                        .iter()
                        .chain(match segment.wrapped_by {
                            segment::WrappedBy::Whitespace(ref chars)
                                if !current_line.is_empty() =>
                            {
                                chars.iter()
                            }
                            _ => [].iter(),
                        })
                        .chain(segment.chars.iter())
                        .cloned()
                        .collect::<Vec<_>>(),
                    controls,
                    f64::INFINITY,
                );
                let segment_layout =
                    lua_handle.text_layout(&segment_text, decoration, char_spacing)?;
                if segment_layout.width > width {
                    if current_line.is_empty() {
                        if segment.chars.len() == 1 {
                            // 1文字も入らない場合はその文字だけで改行する
                            wrapped_lines.push(WrappedLine {
                                chars: segment.chars.clone(),
                                control_index: segment.chars[0].control_index,
                                is_paragraph_end: false,
                            });
                            break 'try_push;
                        } else {
                            // 1文字も入らない場合は1文字ごとに分割する
                            for char_state in segment.chars.into_iter().rev() {
                                segmented.push_front(segment::Segment {
                                    chars: vec![char_state],
                                    wrapped_by: segment::WrappedBy::Overflow,
                                });
                            }
                            segmented.front_mut().unwrap().wrapped_by = segment.wrapped_by;
                            break 'try_push;
                        }
                    }

                    let mut new_line = vec![];
                    std::mem::swap(&mut current_line, &mut new_line);
                    wrapped_lines.push(WrappedLine {
                        control_index: new_line[0].control_index,
                        chars: new_line,
                        is_paragraph_end: false,
                    });
                } else {
                    if let segment::WrappedBy::Whitespace(ref chars) = segment.wrapped_by
                        && !current_line.is_empty()
                    {
                        current_line.extend(chars.clone());
                    }
                    current_line.extend(segment.chars.clone());
                    break 'try_push;
                }
            }
        }
        if !current_line.is_empty() {
            wrapped_lines.push(WrappedLine {
                control_index: current_line[0].control_index,
                chars: current_line,
                is_paragraph_end: true,
            });
        }
    }
    Ok(wrapped_lines)
}

#[expect(clippy::too_many_arguments)]
fn layout_wrapped_lines(
    wrapped_lines: &[WrappedLine],
    controls: &[aviutl2_text_parser::Element],
    lua_handle: &LuaHandle,
    width: usize,
    align: &HorizontalAlign,
    justify: Justify,
    decoration: FullTextDecoration,
    line_spacing: f64,
    char_spacing: f64,
    time: f64,
) -> aviutl2::AnyResult<(Vec<Layout>, f64)> {
    let mut line_y = 0.0_f64;
    let mut layouts: Vec<Layout> = Vec::new();
    for WrappedLine {
        chars: line_chars,
        control_index,
        is_paragraph_end,
    } in wrapped_lines.iter()
    {
        let current_line_text = if line_chars.is_empty() {
            controls_to_text(controls, *control_index)
        } else {
            char_states_to_text(line_chars, controls, f64::INFINITY)
        };
        let visible_current_line_text = char_states_to_text(line_chars, controls, time);
        let horizontal_align = if justify != Justify::No && !is_paragraph_end {
            HorizontalAlign::Justify
        } else {
            *align
        };
        let line_layout = lua_handle.text_layout(&current_line_text, decoration, char_spacing)?;
        if !line_chars
            .iter()
            .any(|char_state| char_state.start_time <= time)
        {
            // 空行の場合は高さだけを確保して次の行へ
            line_y += line_layout.height as f64 + line_spacing;
            continue;
        }
        let visible_line_layout =
            lua_handle.text_layout(&visible_current_line_text, decoration, char_spacing)?;
        let line_center_y = line_y + line_layout.height as f64 / 2.0;
        match horizontal_align {
            HorizontalAlign::Justify if line_chars.len() == 1 => {
                // 1文字しかない場合は両端揃えできないので中央揃えにする
                layouts.push(Layout {
                    content: visible_current_line_text,
                    position: apply_negative_center_delta(
                        width as f64 / 2.0,
                        line_center_y,
                        visible_line_layout,
                    ),
                    size: (
                        visible_line_layout.width as f64,
                        visible_line_layout.height as f64,
                    ),
                });
            }
            HorizontalAlign::Justify => {
                let space_between_chars =
                    (width as f64 - line_layout.width as f64) / (line_chars.len() - 1) as f64;
                let mut emitted_controls = line_chars[0].control_index;
                let mut draw_text = controls_to_text(controls, emitted_controls);
                for c in line_chars.iter() {
                    draw_text.push_str(&controls_between_to_text(
                        controls,
                        emitted_controls,
                        c.control_index,
                    ));
                    if c.start_time <= time {
                        draw_text.push_str(&c.unit.to_string());
                    } else {
                        let control_prefix = controls_to_text(controls, c.control_index);
                        let base_char_layout = lua_handle.text_layout(
                            &format!("{control_prefix} "),
                            decoration,
                            char_spacing,
                        )?;
                        let char_layout = lua_handle.text_layout(
                            &format!("{control_prefix} {}", c.unit),
                            decoration,
                            char_spacing,
                        )?;
                        draw_text.push_str(&format!(
                            "<p+{:.2},+0>",
                            (char_layout.width - base_char_layout.width) as f64,
                        ));
                    }
                    draw_text.push_str(&format!("<p+{:.2},+0>", space_between_chars));
                    emitted_controls = c.control_index;
                }
                let draw_text_layout =
                    lua_handle.text_layout(&draw_text, decoration, char_spacing)?;
                layouts.push(Layout {
                    content: draw_text,
                    position: apply_negative_center_delta(
                        draw_text_layout.width as f64 / 2.0,
                        line_center_y,
                        draw_text_layout,
                    ),
                    size: (
                        draw_text_layout.width as f64,
                        draw_text_layout.height as f64,
                    ),
                });
            }
            HorizontalAlign::Left => {
                layouts.push(Layout {
                    content: visible_current_line_text,
                    position: apply_negative_center_delta(
                        visible_line_layout.width as f64 / 2.0,
                        line_center_y,
                        visible_line_layout,
                    ),
                    size: (
                        visible_line_layout.width as f64,
                        visible_line_layout.height as f64,
                    ),
                });
            }
            HorizontalAlign::Center => {
                layouts.push(Layout {
                    content: visible_current_line_text,
                    position: apply_negative_center_delta(
                        width as f64 / 2.0
                            - (line_layout.width as f64 - visible_line_layout.width as f64) / 2.0,
                        line_center_y,
                        visible_line_layout,
                    ),
                    size: (
                        visible_line_layout.width as f64,
                        visible_line_layout.height as f64,
                    ),
                });
            }
            HorizontalAlign::Right => {
                layouts.push(Layout {
                    content: visible_current_line_text,
                    position: apply_negative_center_delta(
                        width as f64 - line_layout.width as f64
                            + visible_line_layout.width as f64 / 2.0,
                        line_center_y,
                        visible_line_layout,
                    ),
                    size: (
                        visible_line_layout.width as f64,
                        visible_line_layout.height as f64,
                    ),
                });
            }
        }
        line_y += line_layout.height as f64 + line_spacing;
    }
    line_y -= line_spacing;

    Ok((layouts, line_y))
}

pub fn layout(
    LayoutParams {
        lua_callback,
        width,
        align,
        justify,
        segmentation_mode,
        text,
        size,
        line_spacing,
        char_spacing,
        show_speed,
        font,
        color,
        secondary_color,
        outline_size,
        decoration,
        bold,
        italic,
        time,
    }: LayoutParams,
) -> aviutl2::AnyResult<(Vec<u8>, f64, f64)> {
    let lua_handle = LuaHandle::new(lua_callback).context("Failed to create LuaHandle")?;
    let text = aviutl2_text_parser::process_scripts(&text, |script, is_inline| {
        if is_inline {
            lua_handle.evaluate_inline_script(&format!("mes({script})"))
        } else {
            lua_handle.evaluate_inline_script(script)
        }
    })
    .context("Failed to process inline scripts")?;
    let rgb = |value: u32| {
        aviutl2_text_parser::ColorValue::Rgb(
            ((value >> 16) & 0xff) as u8,
            ((value >> 8) & 0xff) as u8,
            (value & 0xff) as u8,
        )
    };
    let initial_controls = vec![
        aviutl2_text_parser::Element::Size {
            size: aviutl2_text_parser::ScalarValue::Absolute(size),
            font: Some(font),
            decoration: Some(aviutl2_text_parser::TextDecoration {
                bold,
                italic,
                strikethrough: false,
            }),
            outline_size: Some(outline_size),
        },
        aviutl2_text_parser::Element::Color {
            code: aviutl2_text_parser::ColorType::Pair(rgb(color), rgb(secondary_color)),
        },
    ];
    let initial_control_index = initial_controls.len();
    let evaluated = evaluate_chars(&text, initial_controls, show_speed);
    tracing::trace!("evaluate_chars {evaluated:?}");
    let mut lines = vec![SourceLine {
        chars: vec![],
        control_index: initial_control_index,
    }];
    for char_state in evaluated.chars {
        if char_state.unit.as_char() == Some('\n') {
            lines.push(SourceLine {
                chars: vec![],
                control_index: char_state.control_index,
            });
        } else {
            lines
                .last_mut()
                .expect("lines always contains at least one line")
                .chars
                .push(char_state);
        }
    }
    tracing::trace!("lines: {lines:#?}");

    let wrapped_lines = build_wrapped_lines(
        &lines,
        &evaluated.controls,
        &lua_handle,
        decoration,
        char_spacing,
        width,
        segmentation_mode,
    )
    .context("Failed to build wrapped lines")?;
    tracing::trace!("wrapped_lines: {wrapped_lines:#?}");

    let width = match justify {
        Justify::No => width,
        Justify::SpecifiedWidth => width,
        Justify::LongestLine => {
            let mut max_line_width = 0;
            for WrappedLine {
                chars: line_chars, ..
            } in wrapped_lines.iter()
            {
                let line_text = char_states_to_text(line_chars, &evaluated.controls, f64::INFINITY);
                let line_layout = lua_handle.text_layout(&line_text, decoration, char_spacing)?;
                if line_layout.width > max_line_width {
                    max_line_width = line_layout.width;
                }
            }
            max_line_width
        }
    };

    let (layouts, height) = layout_wrapped_lines(
        &wrapped_lines,
        &evaluated.controls,
        &lua_handle,
        width,
        &align,
        justify,
        decoration,
        line_spacing,
        char_spacing,
        time,
    )?;
    tracing::trace!("layouts: {layouts:#?}, height: {height}");

    Ok((
        serde_luajit_buffer::serialize_one(&layouts, &Default::default())
            .context("Failed to serialize layouts")?,
        width as f64,
        height,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_center_delta_moves_position_negatively() {
        let position = apply_negative_center_delta(
            100.0,
            50.0,
            TextLayout {
                width: 20,
                height: 30,
                negative_center_x_delta: 4.0,
                negative_center_y_delta: 10.0,
            },
        );

        assert_eq!(position, (96.0, 40.0));
    }
}
