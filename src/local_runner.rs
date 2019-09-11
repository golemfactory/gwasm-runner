use crate::workdir::WorkDir;
use failure::{bail, err_msg, Fallible};
use gwasm_api::TaskDef;
use sp_wasm_engine::prelude::*;
use sp_wasm_engine::sandbox::engine::EngineRef;
use std::fs::OpenOptions;
use std::path::Path;

fn run_local_code(
    engine: EngineRef,
    wasm_path: &Path,
    js_path: &Path,
    task_path: &Path,
    args: Vec<String>,
) -> Fallible<()> {
    let mut sandbox = Sandbox::new_on_engine(engine)?.set_exec_args(args)?;

    sandbox.init()?;
    sandbox.mount("/", "@", NodeMode::Ro)?;
    sandbox.mount(task_path, "/task_dir", NodeMode::Rw)?;

    let cur_dir = std::env::current_dir()?;

    sandbox
        .work_dir(cur_dir.to_string_lossy().as_ref())?
        .run(js_path, wasm_path)?;

    Ok(())
}

fn run_remote_code(
    engine: EngineRef,
    wasm_path: &Path,
    js_path: &Path,
    task_input_path: &Path,
    task_output_path: &Path,
) -> Fallible<()> {
    log::info!(
        "starting work in {} => {}",
        task_input_path.display(),
        task_output_path.display()
    );
    let mut sandbox = Sandbox::new_on_engine(engine)?.set_exec_args(&[
        "exec",
        "/in/task.json",
        "/out/task.json",
    ])?;

    sandbox.init()?;
    sandbox.mount(task_input_path, "/in", NodeMode::Ro)?;
    sandbox.mount(task_output_path, "/out", NodeMode::Rw)?;
    sandbox.work_dir("/in")?.run(js_path, wasm_path)?;
    log::info!(
        "done work in {} => {}",
        task_input_path.display(),
        task_output_path.display()
    );
    Ok(())
}

pub fn run_on_local(wasm_path: &Path, args: &Vec<String>) -> Fallible<()> {
    let engine_ref = Sandbox::init_ejs()?;
    let mut w = WorkDir::new("local")?;

    let js_path = wasm_path.with_extension("js");

    if !js_path.exists() {
        bail!("file not found: {}", js_path.display())
    }

    let output_path = w.split_output()?;
    {
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
    }

    let tasks_path = output_path.join("tasks.json");

    let tasks: Vec<gwasm_api::TaskDef> =
        serde_json::from_reader(OpenOptions::new().read(true).open(tasks_path)?)?;

    let mut input_agg = Vec::new();
    let mut output_agg = Vec::new();
    let merge_path = w.merge_path()?;
    for task in tasks {
        let task_path = w.new_task()?;
        let task_input_path = task_path.join("in");
        let task_output_path = task_path.join("out");

        std::fs::create_dir(&task_input_path)?;
        std::fs::create_dir(&task_output_path)?;

        for blob_path in task.blobs() {
            std::fs::rename(
                &output_path.join(blob_path),
                task_input_path.join(blob_path),
            )?;
        }
        serde_json::to_writer_pretty(
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(task_input_path.join("task.json"))?,
            &task,
        )?;

        run_remote_code(
            engine_ref.clone(),
            wasm_path,
            &js_path,
            &task_input_path,
            &task_output_path,
        )?;

        let output_data: TaskDef = serde_json::from_reader(
            OpenOptions::new()
                .read(true)
                .open(task_output_path.join("task.json"))?,
        )?;

        input_agg.push(task.rebase_to(&task_input_path, &merge_path)?);
        output_agg.push(output_data.rebase_to(&task_output_path, &merge_path)?);
    }
    serde_json::to_writer_pretty(
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(merge_path.join("tasks_input.json"))?,
        &input_agg,
    )?;
    serde_json::to_writer_pretty(
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(merge_path.join("tasks_output.json"))?,
        &output_agg,
    )?;

    {
        let mut merge_args = Vec::new();
        merge_args.push("merge".to_owned());
        merge_args.push("/task_dir/merge/tasks_input.json".to_owned());
        merge_args.push("/task_dir/merge/tasks_output.json".to_owned());
        merge_args.push("--".to_owned());
        merge_args.extend(args.iter().cloned());
        run_local_code(
            engine_ref,
            wasm_path,
            &js_path,
            merge_path.parent().unwrap(),
            merge_args,
        )?;
    }

    Ok(())
}
