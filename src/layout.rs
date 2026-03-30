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
}

fn build_wrapped_lines(
    lines: &[Vec<crate::evaluate_chars::CharState>],
    lua_handle: &LuaHandle,
    decoration: FullTextDecoration,
    char_spacing: f64,
    width: usize,
) -> aviutl2::AnyResult<Vec<Vec<crate::evaluate_chars::CharState>>> {
    let mut wrapped_lines: Vec<Vec<crate::evaluate_chars::CharState>> = Vec::new();
    for line_chars in lines {
        if line_chars.is_empty() {
            wrapped_lines.push(vec![]);
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
                );
                let (segment_width, _) =
                    lua_handle.text_layout(&segment_text, decoration, char_spacing)?;
                if segment_width > width {
                    if current_line.is_empty() {
                        if segment.chars.len() == 1 {
                            // 1文字も入らない場合はその文字だけで改行する
                            wrapped_lines.push(segment.chars.clone());
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
                    wrapped_lines.push(new_line);
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
            wrapped_lines.push(current_line);
        }
    }
    Ok(wrapped_lines)
}

#[expect(clippy::too_many_arguments)]
fn layout_wrapped_lines(
    wrapped_lines: &[Vec<crate::evaluate_chars::CharState>],
    lua_handle: &LuaHandle,
    current_style: &crate::evaluate_chars::CharState,
    width: usize,
    align: &HorizontalAlign,
    justify: bool,
    decoration: FullTextDecoration,
    line_spacing: f64,
    char_spacing: f64,
) -> aviutl2::AnyResult<(Vec<Layout>, f64)> {
    let mut line_y = 0.0_f64;
    let mut layouts: Vec<Layout> = Vec::new();
    let mut current_style = current_style.clone();
    for (i, line_chars) in wrapped_lines.iter().enumerate() {
        let current_line_text = format!(
            "{}{}",
            current_style.to_style_control(),
            crate::evaluate_chars::char_states_to_text(line_chars)
        );
        current_style = line_chars
            .last()
            .map_or(current_style.clone(), |c| c.clone());
        let horizontal_align = if justify && i != wrapped_lines.len() - 1 {
            HorizontalAlign::Justify
        } else {
            *align
        };
        let (line_width, line_height) =
            lua_handle.text_layout(&current_line_text, decoration, char_spacing)?;
        let y = line_y + line_height as f64 / 2.0;
        match horizontal_align {
            HorizontalAlign::Justify if line_chars.len() == 1 => {
                // 1文字しかない場合は両端揃えできないので中央揃えにする
                layouts.push(Layout {
                    content: current_line_text,
                    position: (width as f64 / 2.0 - line_width as f64 / 2.0, y),
                });
            }
            HorizontalAlign::Justify => {
                let char_widths = line_chars
                    .iter()
                    .map(|c| {
                        let text = char_states_to_text(std::iter::once(c));
                        let (w, _) = lua_handle.text_layout(&text, decoration, char_spacing)?;
                        Ok(w as f64)
                    })
                    .collect::<aviutl2::AnyResult<Vec<f64>>>()?;
                let total_char_width: f64 = char_widths.iter().sum();
                let extra_space = width as f64 - total_char_width;
                let space_between_chars = extra_space / (line_chars.len() - 1) as f64;
                let mut x = 0.0;
                for (c, char_width) in line_chars.iter().zip(char_widths.iter()) {
                    let text = char_states_to_text(std::iter::once(c));
                    layouts.push(Layout {
                        content: text,
                        position: (x + char_width / 2.0, y),
                    });
                    x += char_width + space_between_chars;
                }
            }
            HorizontalAlign::Left => {
                layouts.push(Layout {
                    content: current_line_text,
                    position: (line_width as f64 / 2.0, y),
                });
            }
            HorizontalAlign::Center => {
                layouts.push(Layout {
                    content: current_line_text,
                    position: (width as f64 / 2.0, y),
                });
            }
            HorizontalAlign::Right => {
                layouts.push(Layout {
                    content: current_line_text,
                    position: (width as f64 - line_width as f64 / 2.0, y),
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
    )?;

    Ok((
        serde_json::to_string(&layouts).context("Failed to serialize layouts")?,
        height,
    ))
}
