use aviutl2::{anyhow::Context, module::ScriptModuleFunctions, tracing};
use evaluate_chars::{char_states_to_text, evaluate_chars};

mod bisect;
mod budoux;
mod evaluate_chars;
mod lua_handle;

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

enum Align {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(aviutl2::module::FromScriptModuleParam)]
struct LayoutParams {
    lua_callback: String,
    width: usize,
    align: usize,
    justify: bool,
    text: String,
    size: f64,
    letter_spacing: f64,
    line_spacing: f64,
    show_speed: f64,
    font: String,
    color: u32,
    bold: bool,
    italic: bool,
}

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
            bold,
            italic,
        }: LayoutParams,
    ) -> aviutl2::AnyResult<String> {
        let lua_handle =
            lua_handle::LuaHandle::new(lua_callback).context("Failed to create LuaHandle")?;
        let align = match align % 4 {
            0 => Align::Left,
            1 => Align::Center,
            2 => Align::Right,
            3 => Align::Justify,
            _ => unreachable!(),
        };
        let chars = evaluate_chars(
            &text,
            &evaluate_chars::CharState {
                char: ' ',
                bold,
                italic,
                strikethrough: false,
                size,
                color: format!("{:06X}", color),
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
            font: font.clone(),
            start_time: 0.0,
            end_time: None,
        };
        for line_chars in &lines {
            if line_chars.is_empty() {
                base_y += lua_handle.line_height(&current_style)? as f64 + line_spacing;
                continue;
            }
            let chars = budoux::segment_char_states(line_chars);
            aviutl2::ldbg!(chars);
        }

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
