use {
    crate::{
        brass_config::GolemConfig,
        brass_task::TaskBuilder,
        local_runner::run_local_code,
        workdir::{WorkDir, GWASM_APP_INFO},
    },
    app_dirs::{app_dir, AppDataType},
    failure::{bail, format_err, Fallible},
    gwasm_api::prelude::{compute, ComputedTask, GWasmBinary, ProgressUpdate},
    gwasm_dispatcher::TaskDef,
    indicatif::ProgressBar,
    promptly::prompt_default,
    sp_wasm_engine::{prelude::Sandbox, sandbox::engine::EngineRef},
    std::{collections::HashMap, fs::File, fs::OpenOptions, path::PathBuf},
};

const TASK_TYPE: &str = "brass";

struct ProgressUpdater {
    bar: ProgressBar,
    progress: f64,
    num_subtasks: u64,
}

impl ProgressUpdater {
    fn new(num_subtasks: u64) -> Self {
        Self {
            bar: ProgressBar::new(num_subtasks),
            progress: 0.0,
            num_subtasks,
        }
    }
}

impl ProgressUpdate for ProgressUpdater {
    fn update(&mut self, progress: f64) {
        if progress > self.progress {
            let delta = progress - self.progress;
            self.progress = progress;
            self.bar
                .inc((delta * self.num_subtasks as f64).round() as u64);
        }
    }

    fn start(&mut self) {
        self.bar.inc(0)
    }

    fn stop(&mut self) {
        self.bar.finish_and_clear()
    }
}

struct RunnerContext {
    engine_ref: EngineRef,
    golem_config: GolemConfig,
    js_path: PathBuf,
    wasm_path: PathBuf,
    workdir: WorkDir,
}

pub fn run_on_brass(wasm_path: &PathBuf, skip_confirmation: bool, args: &[String]) -> Fallible<()> {
    let golem_config = GolemConfig::from(
        app_dir(AppDataType::UserConfig, &GWASM_APP_INFO, TASK_TYPE)?.join("config.json"),
    )?;

    log::info!("Using: {:#?}", golem_config);
    if !skip_confirmation && !has_user_confirmed(&wasm_path) {
        bail!("Task creation aborted.");
    }

    let workdir = WorkDir::new(TASK_TYPE)?;
    log::info!("Working directory: {}", workdir.base_dir().display());

    let mut context = RunnerContext {
        engine_ref: Sandbox::init_ejs()?,
        golem_config,
        js_path: wasm_path.with_extension("js"),
        wasm_path: wasm_path.to_path_buf(),
        workdir,
    };

    split(args, &mut context)?;
    let computed_task = execute(&mut context)?;
    merge(args, &mut context, computed_task)?;

    log::info!("Task computed!");

    Ok(())
}

fn split(args: &[String], context: &mut RunnerContext) -> Fallible<()> {
    let output_path = context.workdir.split_output()?;
    let mut split_args = Vec::new();
    split_args.push("split".to_owned());
    split_args.push("/task_dir/".to_owned());
    split_args.extend(args.iter().cloned());

    log::debug!("split args: {:?}", split_args);

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
    let wasm_file = std::fs::read(&context.wasm_path)?;
    let js_file = std::fs::read(&context.js_path)?;
    let binary = GWasmBinary {
        js: js_file.as_slice(),
        wasm: wasm_file.as_slice(),
    };

    let builder = TaskBuilder::new(context.workdir.clone(), binary)
        .name(&context.golem_config.name)
        .bid(context.golem_config.bid)
        .budget(context.golem_config.budget)
        .timeout(context.golem_config.task_timeout)
        .subtask_timeout(context.golem_config.subtask_timeout);
    let task = builder.build()?;

    log::debug!("Created task: {:#?}", task);

    log::info!("Starting task computation...");
    let subtask_count = task.options().subtasks().count();
    let address_parts: Vec<&str> = context.golem_config.address.split(':').collect();
    let computed_task = compute(
        &context.golem_config.data_dir,
        address_parts[0],
        address_parts[1].parse()?,
        context.golem_config.net.clone(),
        task,
        ProgressUpdater::new(subtask_count as u64),
    )
    .map_err(|e| log::error!("Task computation failed: {}", e))
    .unwrap();

    log::debug!("Computed task: {:#?}", computed_task);

    Ok(computed_task)
}

fn merge(args: &[String], context: &mut RunnerContext, task: ComputedTask) -> Fallible<()> {
    let merge_path = context.workdir.merge_path()?;
    let mut output_agg = Vec::new();

    let mut id_to_subtask = HashMap::new();
    for subtask in task.subtasks {
        id_to_subtask.insert(subtask.name.clone(), subtask);
    }

    let subtask_order = get_subtask_order(merge_path.join("tasks_input.json"))?;
    log::debug!("subtask order: {:?}", subtask_order);

    // Read subtasks in original order (from tasks_input.json)
    for subtask_id in subtask_order {
        let subtask = id_to_subtask.remove(&subtask_id).unwrap();
        let output_path = context.workdir.base_dir().join(&subtask.name);
        log::debug!("Reading output for subtask: {}", output_path.display());

        for (path, reader) in subtask.data {
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

    log::debug!("merge args: {:?}", merge_args);

    run_local_code(
        context.engine_ref.clone(),
        &context.wasm_path,
        &context.js_path,
        merge_path.parent().unwrap(),
        merge_args,
    )?;

    Ok(())
}

fn has_user_confirmed(wasm_path: &PathBuf) -> bool {
    println!(
        "\nYou are about to create a Brass Golem task with the above parameters. \
         \nThe WASM binary to be used for this task is: {:?}.",
        wasm_path
    );

    return prompt_default("Would you like to proceed?", false)
}

fn get_subtask_order(tasks_input_path: PathBuf) -> Fallible<Vec<String>> {
    let task_defs: Vec<TaskDef> = serde_json::from_reader(File::open(tasks_input_path)?)?;

    let subtask_ids: Vec<String> = task_defs
        .iter()
        .flat_map(|task_def| task_def.outputs())
        .filter_map(|output_path| {
            // Assuming the output path is relative to merge directory (i.e. starts with '..')
            let mut split = output_path.split(std::path::MAIN_SEPARATOR).skip(1);
            split.next()
        })
        .map(|subtask_id| subtask_id.to_string())
        .collect();

    Ok(subtask_ids)
}
