use aviutl2::{anyhow::Context, module::ScriptModuleFunctions, tracing};

mod budoux;
mod evaluate_chars;
mod layout;
mod lending;
mod lua_handle;
mod segment;

#[aviutl2::plugin(ScriptModule)]
struct BudouxMod2 {}

impl aviutl2::module::ScriptModule for BudouxMod2 {
    fn new(_info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        aviutl2::tracing_subscriber::fmt()
            .with_max_level(if cfg!(debug_assertions) {
                tracing::metadata::LevelFilter::TRACE
            } else {
                tracing::metadata::LevelFilter::INFO
            })
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

#[aviutl2::module::functions]
#[allow(clippy::too_many_arguments)]
impl BudouxMod2 {
    fn layout(
        &self,
        params: layout::LayoutParams,
    ) -> aviutl2::AnyResult<(*const u8, usize, f64, f64)> {
        let current = std::time::Instant::now();
        let (buffer, width, height) = layout::layout(params)?;
        tracing::debug!("layout executed in {:?}", current.elapsed());
        let (return_ptr, return_len) = lending::lend_buffer(buffer);
        Ok((return_ptr, return_len, width, height))
    }

    fn free_buffer(&self, ptr: std::ptr::NonNull<u8>) -> aviutl2::AnyResult<()> {
        tracing::debug!("free_buffer called with ptr: {:?}", ptr);
        if lending::release_buffer(ptr.as_ptr()) {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Failed to release buffer for pointer: {:?}",
                ptr
            ))
        }
    }

    fn push_stack(&self, ptr: String, len: usize) -> aviutl2::AnyResult<()> {
        let ptr: usize = ptr.trim_end_matches("LL").parse()?;
        let ptr =
            std::ptr::NonNull::new(ptr as *mut u8).context("push_stack received a null pointer")?;
        let value = unsafe { std::slice::from_raw_parts(ptr.as_ptr(), len) }.to_vec();
        tracing::debug!("push_stack called with {} bytes", value.len());
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
