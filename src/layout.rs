use aviutl2::{anyhow::Context, tracing};

use crate::evaluate_chars::{char_states_to_text, evaluate_chars};
use crate::lua_handle::{FullTextDecoration, LuaHandle};
use crate::segment;

#[derive(Debug, serde::Serialize)]
pub struct Layout {
    content: String,
    position: (f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlign {
    Left,
    Center,
    Right,
    Justify,
}

impl<'a> aviutl2::module::FromScriptModuleParamTable<'a> for HorizontalAlign {
    fn from_param_table(
        param: &'a aviutl2::module::ScriptModuleParamTable,
        key: &str,
    ) -> Option<Self> {
        let value = param.get_int(key);
        match value {
            0 => Some(Self::Left),
            1 => Some(Self::Center),
            2 => Some(Self::Right),
            3 => Some(Self::Justify),
            _ => None,
        }
    }
}

#[derive(aviutl2::module::FromScriptModuleParam)]
pub struct LayoutParams {
    pub lua_callback: String,
    pub width: usize,
    pub align: HorizontalAlign,
    pub justify: bool,
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
    is_paragraph_end: bool,
}

/// Returns wrapped lines paired with a flag indicating whether the line is the last
/// in its explicit paragraph (i.e., followed by `\n` or end-of-input). Justify should
/// not be applied to paragraph-ending lines.
fn build_wrapped_lines(
    lines: &[Vec<crate::evaluate_chars::CharState>],
    lua_handle: &LuaHandle,
    decoration: FullTextDecoration,
    char_spacing: f64,
    width: usize,
) -> aviutl2::AnyResult<Vec<WrappedLine>> {
    let mut wrapped_lines: Vec<WrappedLine> = Vec::new();
    for line_chars in lines {
        if line_chars.is_empty() {
            wrapped_lines.push(WrappedLine {
                chars: vec![],
                is_paragraph_end: true,
            });
            continue;
        }
        let mut segmented = segment::segment(line_chars)
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
                    f64::INFINITY,
                );
                let (segment_width, _) =
                    lua_handle.text_layout(&segment_text, decoration, char_spacing)?;
                if segment_width > width {
                    if current_line.is_empty() {
                        if segment.chars.len() == 1 {
                            // 1文字も入らない場合はその文字だけで改行する
                            wrapped_lines.push(WrappedLine {
                                chars: segment.chars.clone(),
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
    lua_handle: &LuaHandle,
    current_style: &crate::evaluate_chars::CharState,
    width: usize,
    align: &HorizontalAlign,
    justify: bool,
    decoration: FullTextDecoration,
    line_spacing: f64,
    char_spacing: f64,
    time: f64,
) -> aviutl2::AnyResult<(Vec<Layout>, f64)> {
    let mut line_y = 0.0_f64;
    let mut layouts: Vec<Layout> = Vec::new();
    let mut current_style = current_style.clone();
    for WrappedLine {
        chars: line_chars,
        is_paragraph_end,
    } in wrapped_lines.iter()
    {
        let current_line_text = format!(
            "{}{}",
            current_style.to_style_control(),
            crate::evaluate_chars::char_states_to_text(line_chars, f64::INFINITY)
        );
        let visible_current_line_text =
            crate::evaluate_chars::char_states_to_text(line_chars, time);
        current_style = line_chars
            .last()
            .map_or(current_style.clone(), |c| c.clone());
        let horizontal_align = if justify && !is_paragraph_end {
            HorizontalAlign::Justify
        } else {
            *align
        };
        let (line_width, line_height) =
            lua_handle.text_layout(&current_line_text, decoration, char_spacing)?;
        if visible_current_line_text.is_empty() {
            // 空行の場合は高さだけを確保して次の行へ
            line_y += line_height as f64 + line_spacing;
            continue;
        }
        let (visible_line_width, _) =
            lua_handle.text_layout(&visible_current_line_text, decoration, char_spacing)?;
        let y = line_y + line_height as f64 / 2.0;
        match horizontal_align {
            HorizontalAlign::Justify if line_chars.len() == 1 => {
                // 1文字しかない場合は両端揃えできないので中央揃えにする
                layouts.push(Layout {
                    content: visible_current_line_text,
                    position: (width as f64 / 2.0 - line_width as f64 / 2.0, y),
                });
            }
            HorizontalAlign::Justify => {
                let space_between_chars =
                    (width as f64 - line_width as f64) / (line_chars.len() - 1) as f64;
                let mut draw_text = String::new();
                let mut prev_style: Option<crate::evaluate_chars::CharState> = None;
                for c in line_chars.iter() {
                    if prev_style.is_none_or(|prev| !prev.same_style(c)) {
                        draw_text.push_str(&c.to_style_control());
                    }
                    if ((c.start_time)..=(c.end_time.unwrap_or(f64::INFINITY))).contains(&time) {
                        draw_text.push(c.char);
                    } else {
                        let (base_char_width, _) = lua_handle.text_layout(
                            &format!("{} ", c.to_style_control()),
                            decoration,
                            char_spacing,
                        )?;
                        let (char_width, _) = lua_handle.text_layout(
                            &format!("{} {}", c.to_style_control(), c.char),
                            decoration,
                            char_spacing,
                        )?;
                        draw_text.push_str(&format!(
                            "<p+{:.2},+0>",
                            (char_width - base_char_width) as f64,
                        ));
                    }
                    draw_text.push_str(&format!("<p+{:.2},+0>", space_between_chars));
                    prev_style = Some(c.clone());
                }
                let (draw_text_width, _) =
                    lua_handle.text_layout(&draw_text, decoration, char_spacing)?;
                layouts.push(Layout {
                    content: draw_text,
                    position: (draw_text_width as f64 / 2.0, y),
                });
            }
            HorizontalAlign::Left => {
                layouts.push(Layout {
                    content: visible_current_line_text,
                    position: (visible_line_width as f64 / 2.0, y),
                });
            }
            HorizontalAlign::Center => {
                layouts.push(Layout {
                    content: visible_current_line_text,
                    position: (
                        width as f64 / 2.0 - (line_width as f64 - visible_line_width as f64) / 2.0,
                        y,
                    ),
                });
            }
            HorizontalAlign::Right => {
                layouts.push(Layout {
                    content: visible_current_line_text,
                    position: (
                        width as f64 - (line_width as f64 - visible_line_width as f64) / 2.0,
                        y,
                    ),
                });
            }
        }
        line_y += line_height as f64 + line_spacing;
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
) -> aviutl2::AnyResult<(String, f64)> {
    let lua_handle = LuaHandle::new(lua_callback).context("Failed to create LuaHandle")?;
    let chars = evaluate_chars(
        &text,
        &crate::evaluate_chars::CharState {
            char: ' ',
            bold,
            italic,
            strikethrough: false,
            size,
            color: format!("{:06X}", color),
            secondary_color: format!("{:06X}", secondary_color),
            outline_size,
            font: font.clone(),
            start_time: 0.0,
            end_time: None,
        },
        show_speed,
    )
    .context("Failed to evaluate characters")?;
    tracing::trace!("evaluate_chars {chars:?}");
    let lines = chars.into_iter().fold(vec![vec![]], |mut acc, char_state| {
        if char_state.char == '\n' {
            acc.push(vec![]);
        } else {
            acc.last_mut().unwrap().push(char_state);
        }
        acc
    });
    tracing::trace!("lines: {lines:#?}");

    let wrapped_lines = build_wrapped_lines(&lines, &lua_handle, decoration, char_spacing, width)
        .context("Failed to build wrapped lines")?;
    tracing::trace!("wrapped_lines: {wrapped_lines:#?}");

    let current_style: crate::evaluate_chars::CharState = crate::evaluate_chars::CharState {
        char: ' ',
        bold,
        italic,
        strikethrough: false,
        size,
        color: format!("{color:06X}"),
        secondary_color: format!("{secondary_color:06X}"),
        outline_size,
        font: font.clone(),
        start_time: 0.0,
        end_time: None,
    };

    let (layouts, height) = layout_wrapped_lines(
        &wrapped_lines,
        &lua_handle,
        &current_style,
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
        serde_json::to_string(&layouts).context("Failed to serialize layouts")?,
        height,
    ))
}
