use gwr_runtime_api::*;

use anyhow::Context;
use sp_wasm_engine::prelude::NodeMode;
use sp_wasm_engine::sandbox as sp;
use sp_wasm_engine::sandbox::engine::EngineRef;
use sp_wasm_engine::sandbox::load::Bytes;
use std::convert::TryInto;
use std::path::Path;

type Result<T> = anyhow::Result<T>;

#[derive(Clone)]
pub struct SpEngine {
    inner: EngineRef,
}

pub fn engine() -> Result<SpEngine> {
    let inner = sp::Sandbox::init_ejs()
        .map_err(anyhow::Error::msg)
        .context("failed to initialize spider monkey engine")?;
    Ok(SpEngine { inner })
}

pub struct SpSandbox {
    inner: Option<sp::Sandbox>,
}

impl Engine for SpEngine {
    type Sandbox = SpSandbox;

    fn new() -> Result<Self> {
        engine()
    }

    fn sandbox(&self, args: Vec<String>) -> Result<Self::Sandbox> {
        let mut inner = sp::Sandbox::new_on_engine(self.inner.clone())
            .map_err(anyhow::Error::msg)
            .context("engine create")?
            .set_exec_args(args)
            .map_err(anyhow::Error::msg)
            .context("set exec args")?;
        inner
            .init()
            .map_err(anyhow::Error::msg)
            .context("box init")?;
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

#[inline]
fn into_mode(mode: Mode) -> NodeMode {
    match mode {
        Mode::Ro => NodeMode::Ro,
        Mode::Rw => NodeMode::Rw,
        Mode::Wo => NodeMode::Wo,
    }
}

pub struct SpCode {
    wasm: Bytes,
    js: Bytes,
}

impl Sandbox for SpSandbox {
    type Code = SpCode;

    fn mount<PathRef: AsRef<Path>>(&mut self, src: PathRef, des: &str, mode: Mode) -> Result<()> {
        Ok(self
            .inner
            .as_mut()
            .unwrap()
            .mount(src.as_ref(), des, into_mode(mode))
            .map_err(anyhow::Error::msg)
            .context("mount")?)
    }

    fn work_dir(&mut self, dir: &str) -> Result<()> {
        let inner = self
            .inner
            .take()
            .unwrap()
            .work_dir(dir)
            .map_err(anyhow::Error::msg)?;
        self.inner = Some(inner);

        Ok(())
    }

    fn run(self, code: Self::Code) -> Result<()> {
        let _ = self
            .inner
            .unwrap()
            .run(code.js, code.wasm)
            .map_err(anyhow::Error::msg)
            .context("failed to execute emscripten code")?;
        Ok(())
    }

    fn from_wasm_path(&self, wasm_path: &Path) -> Result<Self::Code> {
        log::debug!("loading wasm: {}", wasm_path.display());
        let js_path = wasm_path.with_extension("js");

        let wasm = wasm_path.try_into()?;
        let js = js_path.as_path().try_into()?;

        Ok(SpCode { wasm, js })
    }
}
