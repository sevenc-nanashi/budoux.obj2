use aviutl2::{anyhow::Context, tracing};

use crate::evaluate_chars::{char_states_to_text, evaluate_chars};
use crate::lua_handle::{FullTextDecoration, LuaHandle};
use crate::segment;

#[derive(Debug, serde::Serialize)]
pub struct Layout {
    content: String,
    position: (f64, f64),
}

pub enum HorizontalAlign {
    Left,
    Center,
    Right,
    Justify,
}

pub enum VerticalAlign {
    Top,
    Middle,
    Bottom,
}

pub struct Align {
    pub horizontal: HorizontalAlign,
    pub vertical: VerticalAlign,
}

impl<'a> aviutl2::module::FromScriptModuleParamTable<'a> for Align {
    fn from_param_table(
        param: &'a aviutl2::module::ScriptModuleParamTable,
        key: &str,
    ) -> Option<Self> {
        let value = param.get_int(key);
        let horizontal = match value % 4 {
            0 => HorizontalAlign::Left,
            1 => HorizontalAlign::Center,
            2 => HorizontalAlign::Right,
            3 => HorizontalAlign::Justify,
            _ => unreachable!(),
        };
        let vertical = match value / 4 % 3 {
            0 => VerticalAlign::Top,
            1 => VerticalAlign::Middle,
            2 => VerticalAlign::Bottom,
            _ => unreachable!(),
        };
        Some(Self {
            horizontal,
            vertical,
        })
    }
}

#[derive(aviutl2::module::FromScriptModuleParam)]
pub struct LayoutParams {
    pub lua_callback: String,
    pub width: usize,
    pub align: Align,
    pub justify: bool,
    pub text: String,
    pub size: f64,
    pub letter_spacing: f64,
    pub line_spacing: f64,
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
    current_style: &crate::evaluate_chars::CharState,
    decoration: FullTextDecoration,
    line_spacing: f64,
    width: usize,
) -> aviutl2::AnyResult<Vec<Vec<crate::evaluate_chars::CharState>>> {
    let mut base_y = 0.0_f64;
    let mut wrapped_lines: Vec<Vec<crate::evaluate_chars::CharState>> = Vec::new();
    for line_chars in lines {
        if line_chars.is_empty() {
            base_y += lua_handle.line_height(current_style, decoration)? as f64 + line_spacing;
            continue;
        }
        let mut segmented = segment::segment(line_chars)
            .into_iter()
            .collect::<std::collections::VecDeque<_>>();
        let mut available_width = width as f64;
        let mut current_line = vec![];
        tracing::debug!("Processing line: {line_chars:#?}");
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
                let (segment_width, _) = lua_handle.text_layout(&segment_text, decoration)?;
                if segment_width as f64 > available_width {
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
                    available_width = width as f64;
                } else {
                    if let segment::WrappedBy::Whitespace(ref chars) = segment.wrapped_by
                        && !current_line.is_empty()
                    {
                        current_line.extend(chars.clone());
                    }
                    current_line.extend(segment.chars.clone());
                    available_width -= segment_width as f64;
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

pub fn layout(
    LayoutParams {
        lua_callback,
        width,
        align,
        justify,
        text,
        size,
        letter_spacing,
        line_spacing,
        show_speed,
        font,
        color,
        secondary_color,
        outline_size,
        decoration,
        bold,
        italic,
    }: LayoutParams,
) -> aviutl2::AnyResult<String> {
    let lua_handle =
        LuaHandle::new(lua_callback).context("Failed to create LuaHandle")?;
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
    tracing::debug!("evaluate_chars {chars:?}");
    let lines = chars.into_iter().fold(vec![vec![]], |mut acc, char_state| {
        if char_state.char == '\n' {
            acc.push(vec![]);
        } else {
            acc.last_mut().unwrap().push(char_state);
        }
        acc
    });
    tracing::debug!("lines: {lines:#?}");
    let mut base_y = 0.0_f64;
    let mut layouts: Vec<Layout> = Vec::new();

    let mut current_style: crate::evaluate_chars::CharState = crate::evaluate_chars::CharState {
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

    let wrapped_lines = build_wrapped_lines(
        &lines,
        &lua_handle,
        &current_style,
        decoration,
        line_spacing,
        width,
    )?;
    tracing::debug!("wrapped_lines: {wrapped_lines:#?}");

    Ok(serde_json::to_string(&layouts)?)
}
