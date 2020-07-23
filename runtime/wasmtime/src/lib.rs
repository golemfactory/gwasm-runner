use gwr_runtime_api::*;

use ::wasmtime as w;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::Path;

type Result<T> = anyhow::Result<T>;

pub fn engine() -> Result<WtEngine> {
    let mut config = w::Config::default();
    config
        .cranelift_opt_level(w::OptLevel::Speed)
        .debug_info(false);
    let engine = w::Engine::new(&config);
    Ok(WtEngine { engine })
}

#[derive(Clone)]
pub struct WtEngine {
    engine: w::Engine,
}

impl Engine for WtEngine {
    type Sandbox = WtBox;

    fn new() -> Result<Self> {
        engine()
    }

    fn sandbox(&self, args: Vec<String>) -> Result<Self::Sandbox> {
        let store = w::Store::new(&self.engine);
        let mounts = Default::default();
        let mut my_args = vec!["self".to_owned()];
        my_args.extend(args);
        Ok(WtBox {
            store,
            args: my_args,
            mounts,
        })
    }

    #[inline]
    fn supports_overlay_mount(&self) -> bool {
        false
    }

    #[inline]
    fn supports_workdir(&self) -> bool {
        false
    }
}

pub struct WtBox {
    store: w::Store,
    args: Vec<String>,
    mounts: Vec<(String, File)>,
}

impl Sandbox for WtBox {
    type Code = WtCode;

    fn mount<PathRef: AsRef<Path>>(&mut self, src: PathRef, des: &str, _mode: Mode) -> Result<()> {
        self.mounts.push((
            des.to_owned(),
            std::fs::OpenOptions::new().read(true).open(src)?,
        ));
        Ok(())
    }

    fn work_dir(&mut self, _dir: &str) -> Result<()> {
        Err(anyhow::anyhow!("unimplemented work dir"))
    }

    fn run(self, code: Self::Code) -> Result<()> {
        log::debug!(
            "starting: args=[{:?}]; mounts=[{:?}] ",
            self.args,
            self.mounts
        );
        let module = code.module;

        let mut modules = module
            .imports()
            .iter()
            .map(|e| e.module())
            .collect::<HashSet<_>>();

        let mut deps = HashMap::new();
        if modules.remove("wasi_unstable") {
            let snapshot0 = wasmtime_wasi::old::snapshot_0::create_wasi_instance(
                &self.store,
                self.mounts.as_ref(),
                self.args.as_ref(),
                &[("RUST_BACKTRACE".to_string(), "full".to_string())],
            )?;
            deps.insert("wasi_unstable", snapshot0);
        }

        if modules.remove("wasi_snapshot_preview1") {
            let runtime = wasmtime_wasi::create_wasi_instance(
                &self.store,
                self.mounts.as_ref(),
                self.args.as_ref(),
                &[("RUST_BACKTRACE".to_string(), "full".to_string())],
            )?;
            deps.insert("wasi_snapshot_preview1", runtime);
        }

        if !modules.is_empty() {
            anyhow::bail!("missing modules {:?}", modules);
        }

        let externs = module
            .imports()
            .iter()
            .map(|item| {
                deps.get(item.module())
                    .unwrap()
                    .find_export_by_name(item.name())
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("symbol not found: {:?}", item))
            })
            .collect::<Result<Vec<_>>>()?;

        log::debug!("resolved={}", externs.len());

        /*
                    let wasi = wasmtime_wasi::create_wasi_instance(
                        &self.store, self.mounts.as_ref(), self.args.as_ref(), &vec![])?;
        */
        let instnace = w::Instance::new(&self.store, &module, externs.as_ref())?;

        let f = instnace
            .find_export_by_name("_start")
            .unwrap()
            .func()
            .unwrap();
        let _result = f.borrow().call(&[])?;
        Ok(())
    }

    fn from_wasm_path(&self, path: &Path) -> Result<Self::Code> {
        let data = std::fs::read(path)?;
        let module = w::Module::new(&self.store, data.as_ref())?;
        Ok(WtCode { module })
    }
}

pub struct WtCode {
    module: w::Module,
}
