mod config;
mod runner;
mod task;

use gwr_backend::rt::Engine;
use runner::RunnerContext;
use std::path::Path;

pub trait BrassEngine: Engine {
    fn context_from_path(self, wasm_path: &Path) -> anyhow::Result<RunnerContext<Self>>;
}

use gwr_backend::Flags;
pub use runner::run;

#[derive(Debug, Clone)]
pub struct BrassBackend {}

impl BrassBackend {
    pub fn parse_url(url: &str) -> anyhow::Result<Option<Self>> {
        Ok(match url {
            "Golem" | "Brass" | "BrassGolem" | "GolemBrass" => Some(BrassBackend {}),
            _ => None,
        })
    }

    pub fn run<E: BrassEngine>(
        &self,
        engine: E,
        flags: &Flags,
        wasm_path: &Path,
        args: &[String],
    ) -> anyhow::Result<()> {
        run(engine, wasm_path, flags.skip_confirmation, args)
    }
}
