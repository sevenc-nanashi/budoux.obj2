use aviutl2::{anyhow::Context, module::ScriptModuleFunctions, tracing};
use evaluate_chars::{char_states_to_text, evaluate_chars};
use lua_handle::FullTextDecoration;

mod bisect;
mod budoux;
mod evaluate_chars;
mod lua_handle;
mod segment;

#[aviutl2::plugin(ScriptModule)]
struct BudouxMod2 {}

impl aviutl2::module::ScriptModule for BudouxMod2 {
    fn new(_info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        aviutl2::tracing_subscriber::fmt()
            .with_max_level(aviutl2::tracing::Level::DEBUG)
            .event_format(aviutl2::logger::AviUtl2Formatter)
            .with_writer(aviutl2::logger::AviUtl2LogWriter)
            .init();

        Ok(Self {})
    }
    fn plugin_info(&self) -> aviutl2::module::ScriptModuleTable {
        aviutl2::module::ScriptModuleTable {
            information: "budoux.mod2 / Internal Module".to_string(),
            functions: Self::functions(),
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct Layout {
    content: String,
    position: (f64, f64),
}

enum HorizontalAlign {
    Left,
    Center,
    Right,
    Justify,
}

enum VerticalAlign {
    Top,
    Middle,
    Bottom,
}
struct Align {
    horizontal: HorizontalAlign,
    vertical: VerticalAlign,
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
struct LayoutParams {
    lua_callback: String,
    width: usize,
    align: Align,
    justify: bool,
    text: String,
    size: f64,
    letter_spacing: f64,
    line_spacing: f64,
    show_speed: f64,
    font: String,
    color: u32,
    secondary_color: u32,
    outline_size: f64,
    decoration: FullTextDecoration,
    bold: bool,
    italic: bool,
}

// NOTE: 0.15

#[aviutl2::module::functions]
#[allow(clippy::too_many_arguments)]
impl BudouxMod2 {
    fn layout(
        &self,
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
            lua_handle::LuaHandle::new(lua_callback).context("Failed to create LuaHandle")?;
        let chars = evaluate_chars(
            &text,
            &evaluate_chars::CharState {
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

        let mut current_style: evaluate_chars::CharState = evaluate_chars::CharState {
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

        let mut wrapped_lines: Vec<Vec<evaluate_chars::CharState>> = Vec::new();
        for line_chars in &lines {
            if line_chars.is_empty() {
                base_y += lua_handle.line_height(&current_style, decoration)? as f64 + line_spacing;
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

        tracing::debug!("wrapped_lines: {wrapped_lines:#?}");

        Ok(serde_json::to_string(&layouts)?)
    }

    fn push_stack(&self, value: String) -> aviutl2::AnyResult<()> {
        tracing::debug!("push_stack called with value: {:?}", value);
        lua_handle::push_return_stack(value).context("Failed to push to return stack")?;
        Ok(())
    }

    fn push_stack_error(&self, error: String) -> aviutl2::AnyResult<()> {
        tracing::debug!("push_stack_error called with error: {:?}", error);
        lua_handle::push_return_stack_error(error)
            .context("Failed to push error to return stack")?;
        Ok(())
    }
}

aviutl2::register_script_module!(BudouxMod2);
