use {
    crate::{local_runner::run_local_code, task::TaskBuilder, workdir::WorkDir},
    failure::Fallible,
    gwasm_brass_api::prelude::{compute, GWasmBinary, Net, ProgressUpdate},
    sp_wasm_engine::{prelude::Sandbox, sandbox::engine::EngineRef},
    std::{
        fs::File,
        io::Read,
        path::{Path, PathBuf},
    },
};

struct ProgressTracker;

impl ProgressUpdate for ProgressTracker {
    fn update(&mut self, progress: f64) {
        println!("Current progress = {}", progress);
    }
}

const TASK_TYPE: &str = "brass";

pub fn run_on_brass(wasm_path: &PathBuf, args: &[String]) -> Fallible<()> {
    let engine_ref = Sandbox::init_ejs()?;
    let mut workdir = WorkDir::new(TASK_TYPE)?;

    split(&engine_ref, &mut workdir, &args, wasm_path);
    execute(wasm_path, workdir.clone());
    merge(workdir.merge_path()?);

    Ok(())
}

fn split(
    engine_ref: &EngineRef,
    workdir: &mut WorkDir,
    args: &[String],
    wasm_path: &PathBuf,
) -> Fallible<()> {
    let js_path = wasm_path.with_extension("js");

    let output_path = workdir.split_output()?;
    let mut split_args = Vec::new();
    split_args.push("split".to_owned());
    split_args.push("/task_dir/".to_owned());
    split_args.extend(args.iter().cloned());
    eprintln!("work dir: {}", output_path.display());

    run_local_code(
        engine_ref.clone(),
        wasm_path,
        &js_path,
        &output_path,
        split_args,
    )?;

    Ok(())
}

fn execute(wasm_path: &PathBuf, workdir: WorkDir) -> Fallible<()> {
    let js_path = wasm_path.with_extension("js");

    let wasm_file = read_file(wasm_path)?;
    let js_file = read_file(&js_path)?;
    let binary = GWasmBinary {
        js: js_file.as_slice(),
        wasm: wasm_file.as_slice(),
    };

    let builder = TaskBuilder::new(workdir, binary);
    let task = builder.build()?;

    compute(
        Path::new("/home/kuba/golem-files/issues/gwasm-runner-brass/requestor"),
        "127.0.0.1",
        61004,
        Net::TestNet,
        task,
        ProgressTracker,
    )?;

    Ok(())
}

fn merge(merge_dir: PathBuf) {}

fn read_file(source: &PathBuf) -> Fallible<Vec<u8>> {
    let mut buffer = Vec::new();
    File::open(source)?.read_to_end(&mut buffer)?;
    return Ok(buffer);
}
