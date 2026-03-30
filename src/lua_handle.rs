use anyhow::Context;

type LuaCallback = unsafe extern "C" fn(*const std::os::raw::c_char) -> ();
pub struct LuaHandle {
    callback: LuaCallback,
}
unsafe impl Send for LuaHandle {}

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
    TextLayout {
        text: String,
        decoration: FullTextDecoration,
        char_spacing: f64,
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
    fn from_param_table(
        param: &'a aviutl2::module::ScriptModuleParamTable,
        key: &str,
    ) -> Option<Self> {
        use serde::Deserialize;
        use serde::de::IntoDeserializer;
        let value = param.get_int(key);
        let deserializer: serde::de::value::I32Deserializer<serde::de::value::Error> =
            value.into_deserializer();
        Self::deserialize(deserializer).ok()
    }
}

static LAYOUT_CACHE: std::sync::LazyLock<dashmap::DashMap<u64, (usize, usize)>> =
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
    ) -> anyhow::Result<(usize, usize)> {
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

        LAYOUT_CACHE.insert(cache_key, (result.width, result.height));
        Ok((result.width, result.height))
    }
}
