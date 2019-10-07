use {
    crate::{
        local_runner::run_local_code,
        task::TaskBuilder,
        workdir::{WorkDir, GWASM_APP_INFO},
    },
    app_dirs::{app_dir, AppDataType, AppInfo},
    failure::Fallible,
    gwasm_api::TaskDef,
    gwasm_brass_api::prelude::{compute, ComputedTask, GWasmBinary, Net, ProgressUpdate},
    serde::{Deserialize, Serialize},
    sp_wasm_engine::{prelude::Sandbox, sandbox::engine::EngineRef},
    std::{
        fs::{File, OpenOptions},
        io::Read,
        path::{Path, PathBuf},
        str::FromStr,
    },
};

struct ProgressTracker;

impl ProgressUpdate for ProgressTracker {
    fn update(&mut self, progress: f64) {
        println!("Current progress = {}", progress);
    }
}

const TASK_TYPE: &str = "brass";
pub const GOLEM_APP_INFO: AppInfo = AppInfo {
    name: "golem",
    author: "Golem Factory",
};

struct RunnerContext {
    engine_ref: EngineRef,
    golem_config: GolemConfig,
    js_path: PathBuf,
    wasm_path: PathBuf,
    workdir: WorkDir,
}

#[derive(Debug, Deserialize, Serialize)]
struct GolemConfig {
    address: String,
    bid: f64,
    data_dir: PathBuf,
    name: String,
    net: String,
}

impl GolemConfig {
    fn from(config_path: PathBuf) -> Fallible<GolemConfig> {
        if config_path.exists() {
            let user_config: GolemConfig = serde_json::from_reader(File::open(config_path)?)?;
            return Ok(user_config);
        }
        Ok(GolemConfig::default())
    }
}

impl Default for GolemConfig {
    fn default() -> GolemConfig {
        GolemConfig {
            address: String::from("127.0.0.1:61000"),
            bid: 1.0,
            data_dir: app_dir(AppDataType::UserData, &GOLEM_APP_INFO, "default").unwrap(),
            name: String::from("gwasm-task"),
            net: String::from("testnet"),
        }
    }
}

pub fn run_on_brass(wasm_path: &PathBuf, args: &[String]) -> Fallible<()> {
    let golem_config = GolemConfig::from(
        app_dir(AppDataType::UserConfig, &GWASM_APP_INFO, TASK_TYPE)?.join("config.json"))?;
    let workdir = WorkDir::new(TASK_TYPE)?;

    log::info!("Working directory: {}", workdir.base_dir().display());
    log::info!("Using {:#?}", golem_config);

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
    let wasm_file = read_file(&context.wasm_path)?;
    let js_file = read_file(&context.js_path)?;
    let binary = GWasmBinary {
        js: js_file.as_slice(),
        wasm: wasm_file.as_slice(),
    };

    let builder = TaskBuilder::new(context.workdir.clone(), binary);
    let task = builder.build()?;

    log::debug!("Created task: {:#?}", task);

    let address_parts: Vec<&str> = context.golem_config.address.split(":").collect();
    let computed_task = compute(
        &context.golem_config.data_dir,
        address_parts[0],
        address_parts[1].parse()?,
        Net::from_str(context.golem_config.net.as_str())?,
        task,
        ProgressTracker,
    )
    .map_err(|e| log::error!("Task computation failed: {}", e))
    .unwrap();

    log::debug!("Computed task: {:#?}", computed_task);

    Ok(computed_task)
}

fn merge(args: &[String], context: &mut RunnerContext, task: ComputedTask) -> Fallible<()> {
    let merge_path = context.workdir.merge_path()?;
    let mut output_agg = Vec::new();

    for subtask in task.subtasks.into_iter() {
        let output_path = context.workdir.base_dir().join(subtask.name);
        log::debug!("Reading output for subtask: {}", output_path.display());

        for (path, reader) in subtask.data.into_iter() {
            if path.ends_with("task.json") {
                let output_data: TaskDef = serde_json::from_reader(reader)?;
                output_agg.push(output_data.rebase_to(&output_path, &merge_path)?);
            }
        }
    }

    log::debug!("output_agg: {:?}", output_agg);

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

fn read_file(source: &PathBuf) -> Fallible<Vec<u8>> {
    let mut buffer = Vec::new();
    File::open(source)?.read_to_end(&mut buffer)?;
    return Ok(buffer);
}
