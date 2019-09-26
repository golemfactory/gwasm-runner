use {
    crate::workdir::WorkDir,
    gwasm_brass_api::prelude::*,
    std::{
        fs::File,
        io::Read,
        path::PathBuf,
    },
    failure::Fallible,
};

const TASK_TYPE: &str = "brass";

pub fn run_on_brass(wasm_path: &PathBuf, args: &[String]) -> Fallible<()> {
    let mut workdir = WorkDir::new(TASK_TYPE)?;

    split(workdir.split_output()?);
    execute(wasm_path, &workdir);
    merge(workdir.merge_path()?);

    Ok(())
}

fn split(split_dir: PathBuf) {}

fn execute(wasm_path: &PathBuf, workdir: &WorkDir) -> Fallible<()> {
    let js_path = wasm_path.with_extension("js");

    let binary = GWasmBinary{
        js: read_file(wasm_path)?.as_slice(),
        wasm: read_file(&js_path)?.as_slice(),
    };

    Ok(())
}

fn merge(merge_dir: PathBuf) {}

fn read_file(source: &PathBuf) -> Fallible<Vec<u8>> {
    let mut buffer = Vec::new();
    File::open(source)?.read_to_end(&mut buffer)?;
    return Ok(buffer);
}
