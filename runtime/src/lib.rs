use std::path::Path;

type Result<T> = anyhow::Result<T>;

#[derive(Clone, Copy)]
pub enum Mode {
    Ro,
    Rw,
    Wo,
}

pub trait Engine: Clone {
    type Sandbox: Sandbox;

    fn new() -> Result<Self>;

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
