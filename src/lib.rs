use std::ptr::NonNull;

use aviutl2::{
    anyhow::{self, Context},
    module::ScriptModuleFunctions,
    tracing,
};

mod evaluate_chars;

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

type LuaCallback = unsafe extern "C" fn(*const std::os::raw::c_char) -> ();
struct LuaHandle {
    callback: LuaCallback,
}
unsafe impl Send for LuaHandle {}

struct TextState {
    bold: bool,
    italic: bool,
    strike: bool,
    size: f64,
    font: String,
}

impl LuaHandle {
    fn new(lua_callback: String) -> anyhow::Result<Self> {
        let lua_callback: usize = lua_callback.trim_end_matches("LL").parse()?;
        let callback: LuaCallback = unsafe { std::mem::transmute(lua_callback) };
        Ok(Self { callback })
    }
    pub fn text_width(&self, text: &str) -> anyhow::Result<(usize, usize)> {
        let c_string = std::ffi::CString::new(text)?;
        unsafe { (self.callback)(c_string.as_ptr()) };
        let result = pop_return_stack().context("Failed to pop from return stack")?;
        result
            .split_once(',')
            .map(|(w, h)| {
                let width = w.trim().parse().context("Failed to parse width")?;
                let height = h.trim().parse().context("Failed to parse height")?;
                Ok((width, height))
            })
            .context("Failed to split result")?
    }
}

static RETURN_STACK: std::sync::Mutex<Vec<Result<String, String>>> =
    std::sync::Mutex::new(Vec::new());
fn pop_return_stack() -> anyhow::Result<String> {
    let mut stack = RETURN_STACK.lock().unwrap();
    stack
        .pop()
        .context("Return stack is empty")?
        .map_err(|e| anyhow::anyhow!("Lua callback error: {e}"))
}

#[aviutl2::module::functions]
#[allow(clippy::too_many_arguments)]
impl BudouxMod2 {
    fn layout(
        &self,
        lua_callback: String,
        text: String,
        size: f64,
        letter_spacing: f64,
        line_spacing: f64,
        show_speed: f64,
        font: String,
        color: u32,
        bold: bool,
        italic: bool,
        strike: bool,
    ) -> aviutl2::AnyResult<()> {
        let lua_handle = LuaHandle::new(lua_callback).context("Failed to create LuaHandle")?;
        tracing::debug!("LuaHandle created successfully");
        let text = "Hello, AviUtl!";
        let (width, height) = lua_handle
            .text_width(text)
            .context("Failed to get text width")?;
        tracing::debug!("Text width obtained: width={}, height={}", width, height);
        Ok(())
    }

    fn push_stack(&self, value: String) -> aviutl2::AnyResult<()> {
        tracing::debug!("push_stack called with value: {:?}", value);
        let mut stack = RETURN_STACK.lock().unwrap();
        stack.push(Ok(value));
        Ok(())
    }

    fn push_stack_error(&self, error: String) -> aviutl2::AnyResult<()> {
        tracing::debug!("push_stack_error called with error: {:?}", error);
        let mut stack = RETURN_STACK.lock().unwrap();
        stack.push(Err(error));
        Ok(())
    }
}

aviutl2::register_script_module!(BudouxMod2);
