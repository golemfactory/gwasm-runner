#![allow(unused)]
use std::path::Path;
use std::sync::Arc;

type Result<T> = anyhow::Result<T>;

#[derive(Clone, Copy)]
pub enum Mode {
    Ro,
    Rw,
    Wo,
}

pub trait Engine: Clone {
    type Sandbox: Sandbox;

    fn sandbox(&self, args: Vec<String>) -> Result<Self::Sandbox>;

    fn supports_overlay_mount(&self) -> bool;

    fn supports_workdir(&self) -> bool;
}

pub trait Sandbox {
    type Code;

    fn mount<PathRef: AsRef<Path>>(&mut self, src: PathRef, des: &str, mode: Mode) -> Result<()>;

    fn work_dir(&mut self, dir: &str) -> Result<()>;

    fn run(self, code: Self::Code) -> Result<()>;

    fn from_wasm_path(&self, path: &Path) -> Result<Self::Code>;
}

pub fn engine() -> impl Engine {
    wasmtime::engine()
}

mod wasmtime {
    use super::*;
    use ::wasmtime as w;
    use std::collections::{HashMap, HashSet};
    use std::fs::File;
    use std::path::PathBuf;

    pub fn engine() -> impl Engine {
        let mut config = w::Config::default();
        config
            .cranelift_opt_level(w::OptLevel::Speed)
            .debug_info(false);
        let engine = w::Engine::new(&config);
        WtEngine { engine }
    }

    #[derive(Clone)]
    struct WtEngine {
        engine: w::Engine,
    }

    impl Engine for WtEngine {
        type Sandbox = WtBox;

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

    struct WtBox {
        store: w::Store,
        args: Vec<String>,
        mounts: Vec<(String, File)>,
    }

    impl Sandbox for WtBox {
        type Code = WtCode;

        fn mount<PathRef: AsRef<Path>>(
            &mut self,
            src: PathRef,
            des: &str,
            _mode: Mode,
        ) -> Result<()> {
            self.mounts.push((
                des.to_owned(),
                std::fs::OpenOptions::new().read(true).open(src)?,
            ));
            Ok(())
        }

        fn work_dir(&mut self, dir: &str) -> Result<()> {
            Err(anyhow::anyhow!("unimplemented work dir"))
        }

        fn run(self, code: Self::Code) -> Result<()> {
            eprintln!("starting: [{:?}] [{:?}]", self.mounts, self.args);
            let module = code.module;

            let mut modules = module
                .imports()
                .into_iter()
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
                .into_iter()
                .map(|item| {
                    deps.get(item.module())
                        .unwrap()
                        .find_export_by_name(item.name())
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("symbol not found: {:?}", item))
                })
                .collect::<Result<Vec<_>>>()?;

            eprintln!("resolved={}", externs.len());

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

    struct WtCode {
        module: w::Module,
    }
}

#[cfg(feature = "emscripten-engine")]
mod spwasm {
    use super::*;
    use sp_wasm_engine::prelude::NodeMode;
    use sp_wasm_engine::sandbox as sp;
    use sp_wasm_engine::sandbox::engine::EngineRef;
    use sp_wasm_engine::sandbox::load::Bytes;
    use std::convert::TryInto;
    use std::error::Error;

    #[derive(Clone)]
    struct SpEngine {
        inner: EngineRef,
    }

    struct SpSandbox {
        inner: Option<sp::Sandbox>,
    }

    impl Engine for SpEngine {
        type Sandbox = SpSandbox;

        fn sandbox(&self, args: Vec<String>) -> Result<Self::Sandbox> {
            let mut inner = sp::Sandbox::new_on_engine(self.inner.clone())?.set_exec_args(args)?;

            inner.init()?;

            Ok(SpSandbox { inner: Some(inner) })
        }

        #[cfg(windows)]
        fn supports_overlay_mount(&self) -> bool {
            false
        }

        #[cfg(unix)]
        fn supports_overlay_mount(&self) -> bool {
            true
        }

        fn supports_workdir(&self) -> bool {
            true
        }
    }

    impl From<NodeMode> for Mode {
        fn from(n: NodeMode) -> Self {
            match n {
                NodeMode::Ro => Mode::Ro,
                NodeMode::Rw => Mode::Rw,
                NodeMode::Wo => Mode::Wo,
            }
        }
    }

    #[inline]
    fn into_mode(mode: Mode) -> NodeMode {
        match mode {
            Mode::Ro => NodeMode::Ro,
            Mode::Rw => NodeMode::Rw,
            Mode::Wo => NodeMode::Wo,
        }
    }

    struct SpCode {
        wasm: Bytes,
        js: Bytes,
    }

    impl Code for SpCode {
        fn from_wasm_path(wasm_path: &Path) -> Result<Self> {
            let js_path = wasm_path.with_extension("js");

            let wasm = wasm_path.try_into()?;
            let js = js_path.as_path().try_into()?;

            Ok(SpCode { wasm, js })
        }
    }

    impl Sandbox for SpSandbox {
        type Code = SpCode;

        fn mount(&mut self, src: &str, des: &str, mode: Mode) -> Result<()> {
            Ok(self
                .inner
                .as_mut()
                .unwrap()
                .mount(src, des, into_mode(mode))?)
        }

        fn work_dir(&mut self, dir: &str) -> Result<()> {
            let inner = self.inner.take().unwrap().work_dir(dir)?;
            self.inner = Some(inner);

            Ok(())
        }

        fn run(self, code: Self::Code) -> Result<()> {
            let _ = self.inner.unwrap().run(code.wasm, code.js)?;
            Ok(())
        }
    }
}
