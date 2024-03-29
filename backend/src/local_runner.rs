#![allow(unused)]

use crate::rt::{Engine, Mode, Sandbox};
use crate::workdir::WorkDir;
use anyhow::{anyhow, bail, Result as Fallible};
use gwasm_dispatcher::TaskDef;
use std::fs::OpenOptions;
use std::io::BufWriter;
use std::path::{Component, Path, PathBuf};

pub fn run_local_code<E: Engine>(
    engine: E,
    wasm_path: &Path,
    task_path: &Path,
    args: Vec<String>,
) -> Fallible<()> {
    let mut sandbox = engine.sandbox(args)?;

    let mut base = PathBuf::from("/");
    let mut cur_dir = std::env::current_dir()?;

    if engine.supports_overlay_mount() {
        sandbox.mount("/", "@", Mode::Rw)?;
    } else {
        let mut it = cur_dir.components();
        let mut c = it.next();
        while let Some(Component::Prefix(p)) = c {
            base = PathBuf::from(p.as_os_str()).join("/");
            c = it.next();
        }

        cur_dir = PathBuf::from("/hostfs").join(it.as_path());
        sandbox.mount(&base, "/hostfs", Mode::Rw)?;
        if !engine.supports_workdir() {
            sandbox.mount(".", ".", Mode::Rw)?;
        }
    }

    sandbox.mount(task_path, "/task_dir", Mode::Rw)?;

    if engine.supports_workdir() {
        sandbox.work_dir(cur_dir.to_string_lossy().replace('\\', "/").as_ref())?
    }

    let code = sandbox.for_wasm_path(wasm_path)?;

    sandbox.run(code)?;

    Ok(())
}

fn run_remote_code<E: Engine>(
    engine: E,
    wasm_path: &Path,
    task_input_path: &Path,
    task_output_path: &Path,
) -> Fallible<()> {
    log::info!(
        "starting work in {} => {}",
        task_input_path.display(),
        task_output_path.display()
    );
    let mut sandbox = engine.sandbox(vec![
        "exec".to_string(),
        "/in/task.json".to_string(),
        "/out/task.json".to_string(),
    ])?;

    sandbox.mount(task_input_path, "/in", Mode::Ro)?;
    sandbox.mount(task_output_path, "/out", Mode::Rw)?;
    if engine.supports_workdir() {
        sandbox.work_dir("/in")?;
    }

    let code = sandbox.for_wasm_path(wasm_path)?;
    sandbox.run(code)?;

    log::info!(
        "done work in {} => {}",
        task_input_path.display(),
        task_output_path.display()
    );
    Ok(())
}

pub fn run_on_local(engine: impl Engine, wasm_path: &Path, args: &[String]) -> Fallible<()> {
    let mut w = WorkDir::new("local")?;

    let output_path = w.split_output()?;
    {
        let mut split_args = Vec::new();
        split_args.push("split".to_owned());
        split_args.push("/task_dir/".to_owned());
        split_args.extend(args.iter().cloned());
        run_local_code(engine.clone(), wasm_path, &output_path, split_args)?;
    }

    let tasks_path = output_path.join("tasks.json");

    let tasks: Vec<gwasm_dispatcher::TaskDef> =
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
        let task = task.rebase_output("", "../out/");
        serde_json::to_writer_pretty(
            BufWriter::new(
                OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(task_input_path.join("task.json"))?,
            ),
            &task,
        )?;

        run_remote_code(
            engine.clone(),
            wasm_path,
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
        let mut merge_args = vec![
            "merge".to_owned(),
            "/task_dir/merge/tasks_input.json".to_owned(),
            "/task_dir/merge/tasks_output.json".to_owned(),
            "--".to_owned(),
        ];

        merge_args.extend(args.iter().cloned());
        run_local_code(engine, wasm_path, merge_path.parent().unwrap(), merge_args)?;
    }

    Ok(())
}
