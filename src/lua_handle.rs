use anyhow::Context;
use aviutl2::tracing;

type LuaCallback = unsafe extern "C" fn(*const u8, usize);
pub struct LuaHandle {
    callback: LuaCallback,
}
unsafe impl Send for LuaHandle {}

static RETURN_STACK: std::sync::Mutex<Vec<Result<Vec<u8>, String>>> =
    std::sync::Mutex::new(Vec::new());
pub fn push_return_stack(value: Vec<u8>) -> anyhow::Result<()> {
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
    let result_buffer = stack
        .pop()
        .context("Return stack is empty")?
        .map_err(|e| anyhow::anyhow!("Lua callback error: {e}"))?;
    tracing::debug!(
        "pop_return_stack called, result_buffer length: {}",
        result_buffer.len()
    );
    match serde_luajit_buffer::deserialize_one::<T>(&result_buffer, &Default::default()) {
        Ok(value) => Ok(value),
        Err(e) => {
            tracing::error!(
                "Failed to deserialize return value from Lua callback: {:?}",
                e
            );
            Err(anyhow::anyhow!(
                "Failed to deserialize return value: {:?}",
                e
            ))
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum LuaRequest {
    TextLayout {
        text: String,
        decoration: FullTextDecoration,
        char_spacing: f64,
    },
    EvaluateInlineScript {
        script: String,
    },
}

#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    serde_repr::Serialize_repr,
    serde_repr::Deserialize_repr,
    PartialEq,
    Eq,
    Hash,
)]
#[repr(u8)]
pub enum FullTextDecoration {
    #[default]
    Normal = 0,
    Shadow,
    LightShadow,
    Outlined,
    ThinOutlined,
    BoldOutlined,
    SquareOutlined,
}

impl<'a> aviutl2::module::FromScriptModuleParamTable<'a> for FullTextDecoration {
    type Error = serde::de::value::Error;

    fn from_param_table(
        param: &'a aviutl2::module::ScriptModuleParamTable,
        key: &str,
    ) -> Result<FullTextDecoration, aviutl2::module::GetParamError<serde::de::value::Error>> {
        use serde::Deserialize;
        use serde::de::IntoDeserializer;
        let value = param.get_int(key);
        let deserializer: serde::de::value::I32Deserializer<serde::de::value::Error> =
            value.into_deserializer();
        Self::deserialize(deserializer).map_err(aviutl2::module::GetParamError::ConversionError)
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct TextLayout {
    pub width: usize,
    pub height: usize,
    pub negative_center_x_delta: f64,
    pub negative_center_y_delta: f64,
}

static LAYOUT_CACHE: std::sync::LazyLock<dashmap::DashMap<u64, TextLayout>> =
    std::sync::LazyLock::new(dashmap::DashMap::new);

impl LuaHandle {
    pub fn new(lua_callback: String) -> anyhow::Result<Self> {
        let lua_callback: usize = lua_callback.trim_end_matches("LL").parse()?;
        let callback: LuaCallback = unsafe { std::mem::transmute(lua_callback) };
        Ok(Self { callback })
    }
    pub fn text_layout(
        &self,
        styled_text: &str,
        decoration: FullTextDecoration,
        char_spacing: f64,
    ) -> anyhow::Result<TextLayout> {
        let cache_key = {
            // NOTE: さすがに衝突はしないでしょう...
            use xxhash_rust::xxh3::Xxh3;
            let mut hasher = Xxh3::new();
            // せっかくUUIDをもらったので有効活用してあげる
            // https://twitter.com/mimifuwacc/status/2037864289374249321
            hasher.update(b"05d5d995-b7dd-48b3-ab4b-5e210fb1f602");
            hasher.update(styled_text.as_bytes());
            hasher.update(&[decoration as u8]);
            hasher.update(&char_spacing.to_bits().to_le_bytes());
            hasher.digest()
        };
        if let Some(cached) = LAYOUT_CACHE.get(&cache_key) {
            return Ok(*cached);
        }
        let request = LuaRequest::TextLayout {
            text: styled_text.to_string(),
            decoration,
            char_spacing,
        };
        let buffer = serde_luajit_buffer::serialize_one(&request, &Default::default())?;
        unsafe { (self.callback)(buffer.as_ptr(), buffer.len()) };
        let result = pop_return_stack::<TextLayout>().context("Failed to pop from return stack")?;
        tracing::debug!(
            "text_layout result: width={}, height={}, negative_center_delta=({}, {})",
            result.width,
            result.height,
            result.negative_center_x_delta,
            result.negative_center_y_delta,
        );

        LAYOUT_CACHE.insert(cache_key, result);
        Ok(result)
    }

    pub fn evaluate_inline_script(&self, script: &str) -> anyhow::Result<String> {
        let request = LuaRequest::EvaluateInlineScript {
            script: script.to_string(),
        };
        let buffer = serde_luajit_buffer::serialize_one(&request, &Default::default())?;
        unsafe { (self.callback)(buffer.as_ptr(), buffer.len()) };
        let result = pop_return_stack::<String>().context("Failed to pop from return stack")?;
        tracing::debug!("evaluate_inline_script result: {}", result);
        Ok(result)
    }
}
