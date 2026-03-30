use aviutl2::{anyhow::Context, module::ScriptModuleFunctions, tracing};

mod budoux;
mod evaluate_chars;
mod layout;
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
                tracing::metadata::LevelFilter::DEBUG
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
    fn layout(&self, params: layout::LayoutParams) -> aviutl2::AnyResult<(String, f64, f64)> {
        let current = std::time::Instant::now();
        let res = layout::layout(params);
        tracing::debug!("layout executed in {:?}", current.elapsed());
        res
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
