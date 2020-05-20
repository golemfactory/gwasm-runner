mod local_runner;
mod workdir;

use app_dirs::AppInfo;
pub use gwasm_dispatcher as dispatcher;
pub use gwr_runtime_api as rt;
use humantime::Duration;
pub use local_runner::{run_local_code, run_on_local};
use std::path::{Path, PathBuf};
use structopt::StructOpt;
pub use workdir::WorkDir;

#[cfg(feature = "spwasm")]
#[macro_export]
macro_rules! for_spwasm {
    {
        $($it:item)*

    } => {
        $($it)*
    }
}

#[cfg(not(feature = "spwasm"))]
#[macro_export]
macro_rules! for_spwasm {
    {
        $($it:item)*

    } => {}
}

#[cfg(feature = "wasmtime")]
#[macro_export]
macro_rules! for_wasmtime {
    {
        $($it:item)*

    } => {
        $($it)*
    }
}

#[cfg(not(feature = "wasmtime"))]
#[macro_export]
macro_rules! for_wasmtime {
    {
        $($it:item)*

    } => {}
}

for_spwasm! {
    pub use gwr_runtime_spwasm::{SpEngine, SpCode, SpSandbox};
}

for_wasmtime! {
    pub use gwr_runtime_wasmtime::{WtEngine, WtBox, WtCode};
}

const GWASM_APP_INFO: AppInfo = AppInfo {
    name: "g-wasm-runner",
    author: "Golem Factory",
};

pub fn config_path(module: &str) -> anyhow::Result<PathBuf> {
    Ok(app_dirs::app_dir(
        app_dirs::AppDataType::UserConfig,
        &GWASM_APP_INFO,
        module,
    )?)
}

#[derive(StructOpt, Debug, Clone)]
pub struct Flags {
    /// Verbosity level. Add more v's to make app more verbose.
    #[structopt(short, parse(from_occurrences))]
    pub verbose: u8,
    /// Skip confirmation dialogs
    #[structopt(short = "y", long = "assume-yes")]
    pub skip_confirmation: bool,
    /// Set timeout for all tasks (Wasi mode only).
    #[structopt(long, default_value = "3h")]
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct LocalBackend {}

impl LocalBackend {
    pub fn parse_url(url: &str) -> anyhow::Result<Option<Self>> {
        Ok(match url {
            "L" | "Local" | "local" => Some(LocalBackend {}),
            _ => None,
        })
    }

    pub fn run<E: rt::Engine>(
        &self,
        engine: E,
        _flags: &Flags,
        wasm_path: &Path,
        args: &[String],
    ) -> anyhow::Result<()> {
        run_on_local(engine, wasm_path, args)
    }
}
