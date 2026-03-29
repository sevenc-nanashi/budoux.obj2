use anyhow::Context;

type LuaCallback = unsafe extern "C" fn(*const std::os::raw::c_char) -> ();
pub struct LuaHandle {
    callback: LuaCallback,
}
unsafe impl Send for LuaHandle {}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct TextState {
    bold: bool,
    italic: bool,
    strike: bool,
    size: f64,
    font: String,
}

static RETURN_STACK: std::sync::Mutex<Vec<Result<String, String>>> =
    std::sync::Mutex::new(Vec::new());
pub fn push_return_stack(value: String) -> anyhow::Result<()> {
    let mut stack = RETURN_STACK.lock().unwrap();
    stack.push(Ok(value));
    Ok(())
}
pub fn push_return_stack_error(error: String) -> anyhow::Result<()> {
    let mut stack = RETURN_STACK.lock().unwrap();
    stack.push(Err(error));
    Ok(())
}
fn pop_return_stack<T: serde::de::DeserializeOwned>() -> anyhow::Result<T> {
    let mut stack = RETURN_STACK.lock().unwrap();
    let result_json = stack
        .pop()
        .context("Return stack is empty")?
        .map_err(|e| anyhow::anyhow!("Lua callback error: {e}"))?;

    Ok(serde_json::from_str(&result_json)?)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum LuaRequest {
    TextLayout(String),
}

impl LuaHandle {
    pub fn new(lua_callback: String) -> anyhow::Result<Self> {
        let lua_callback: usize = lua_callback.trim_end_matches("LL").parse()?;
        let callback: LuaCallback = unsafe { std::mem::transmute(lua_callback) };
        Ok(Self { callback })
    }
    pub fn text_layout(&self, styled_text: &str) -> anyhow::Result<(usize, usize)> {
        let request = LuaRequest::TextLayout(styled_text.to_string());
        let json = serde_json::to_string(&request)?;
        let c_string = std::ffi::CString::new(json)?;
        unsafe { (self.callback)(c_string.as_ptr()) };
        #[derive(serde::Deserialize)]
        struct ReturnValue {
            width: usize,
            height: usize,
        }
        let result =
            pop_return_stack::<ReturnValue>().context("Failed to pop from return stack")?;
        Ok((result.width, result.height))
    }
    pub fn line_height(&self, style: &crate::evaluate_chars::CharState) -> anyhow::Result<usize> {
        let (_, h1) = self.text_layout(&style.to_style_control())?;
        let (_, h2) = self.text_layout(&format!("{}\n", style.to_style_control(),))?;
        Ok(h2 - h1)
    }
}
