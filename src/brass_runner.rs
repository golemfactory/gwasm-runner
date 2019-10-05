use {
    crate::{local_runner::run_local_code, task::TaskBuilder, workdir::WorkDir},
    failure::Fallible,
    gwasm_api::TaskDef,
    gwasm_brass_api::prelude::{
        compute,
        ComputedTask,
        GWasmBinary,
        Net,
        ProgressUpdate
    },
    sp_wasm_engine::{prelude::Sandbox, sandbox::engine::EngineRef},
    std::{
        fs::{File, OpenOptions},
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

struct RunnerContext {
    engine_ref: EngineRef,
    js_path: PathBuf,
    wasm_path: PathBuf,
    workdir: WorkDir,
}

pub fn run_on_brass(wasm_path: &PathBuf, args: &[String]) -> Fallible<()> {
    let mut context = RunnerContext {
        engine_ref: Sandbox::init_ejs()?,
        wasm_path: wasm_path.to_path_buf(),
        js_path: wasm_path.with_extension("js"),
        workdir: WorkDir::new(TASK_TYPE)?,
    };

    split(args, &mut context);
    let computed_task = execute(&mut context)?;
    merge(args, &mut context, computed_task);

    Ok(())
}

fn split(args: &[String], context: &mut RunnerContext) -> Fallible<()> {
    let output_path = context.workdir.split_output()?;
    let mut split_args = Vec::new();
    split_args.push("split".to_owned());
    split_args.push("/task_dir/".to_owned());
    split_args.extend(args.iter().cloned());
    eprintln!("work dir: {}", output_path.display());

    run_local_code(
        context.engine_ref.clone(),
        &context.wasm_path,
        &context.js_path,
        &output_path,
        split_args,
    )?;

    Ok(())
}

fn execute(context: &mut RunnerContext) -> Fallible<ComputedTask> {
    let wasm_file = read_file(&context.wasm_path)?;
    let js_file = read_file(&context.js_path)?;
    let binary = GWasmBinary {
        js: js_file.as_slice(),
        wasm: wasm_file.as_slice(),
    };

    let builder = TaskBuilder::new(context.workdir.clone(), binary);
    let task = builder.build()?;

    let computed_task = compute(
        Path::new("/home/kuba/golem-files/datadirs/gwasm-brass-runner/requestor"),
        "127.0.0.1",
        61000,
        Net::TestNet,
        task,
        ProgressTracker,
    ).map_err(|e| format!("Task computation failed: {}", e)).unwrap();

    Ok(computed_task)
}

fn merge(args: &[String], context: &mut RunnerContext, task: ComputedTask) -> Fallible<()> {
    let merge_path = context.workdir.merge_path()?;
    let mut output_agg = Vec::new();

    for subtask in task.subtasks.into_iter() {
        let output_path = context.workdir.base_dir().join(subtask.name);
        for (path, reader) in subtask.data.into_iter() {
            if path.ends_with("task.json") {
                let output_data: TaskDef = serde_json::from_reader(reader)?;
                output_agg.push(output_data.rebase_to(&output_path, &merge_path)?);
            }
        }
    }

    serde_json::to_writer_pretty(
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(merge_path.join("tasks_output.json"))?,
        &output_agg,
    )?;

    let mut merge_args = Vec::new();
    merge_args.push("merge".to_owned());
    merge_args.push("/task_dir/merge/tasks_input.json".to_owned());
    merge_args.push("/task_dir/merge/tasks_output.json".to_owned());
    merge_args.push("--".to_owned());
    merge_args.extend(args.iter().cloned());

    run_local_code(
        context.engine_ref.clone(),
        &context.wasm_path,
        &context.js_path,
        merge_path.parent().unwrap(),
        merge_args,
    )?;

    Ok(())
}

fn read_file(source: &PathBuf) -> Fallible<Vec<u8>> {
    let mut buffer = Vec::new();
    File::open(source)?.read_to_end(&mut buffer)?;
    return Ok(buffer);
}
