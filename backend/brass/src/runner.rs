#![allow(unused_imports)]

use crate::BrassEngine;
use std::cell::Cell;
use std::path::Path;
use {
    crate::{config::GolemConfig, task::TaskBuilder},
    gwasm_api::prelude::{compute, ComputedTask, GWasmBinary, ProgressUpdate},
    gwr_backend::{
        config_path, dispatcher::TaskDef, for_spwasm, for_wasmtime, rt, run_local_code, WorkDir,
    },
    indicatif::ProgressBar,
    promptly::prompt_default,
    std::{collections::HashMap, fs::OpenOptions, path::PathBuf},
};

struct ProgressUpdater {
    bar: ProgressBar,
    progress: Cell<f64>,
    num_subtasks: u64,
}

impl ProgressUpdater {
    fn new(num_subtasks: u64) -> Self {
        Self {
            bar: ProgressBar::new(num_subtasks),
            progress: Cell::new(0.0),
            num_subtasks,
        }
    }
}

impl ProgressUpdate for ProgressUpdater {
    fn update(&self, progress: f64) {
        if progress > self.progress.get() {
            let delta = progress - self.progress.get();
            self.progress.set(progress);
            self.bar
                .inc((delta * self.num_subtasks as f64).round() as u64);
        }
    }

    fn start(&self) {
        self.bar.inc(0)
    }

    fn stop(&self) {
        self.bar.finish_and_clear()
    }
}

pub struct RunnerContext<E: rt::Engine> {
    engine_ref: E,
    golem_config: GolemConfig,
    js_path: PathBuf,
    wasm_path: PathBuf,
    workdir: WorkDir,
}
for_spwasm! {
    const TASK_TYPE: &str = "brass";

    impl BrassEngine for gwr_backend::SpEngine {
        fn context_from_path(self, wasm_path: &Path) -> anyhow::Result<RunnerContext<Self>> {
            let golem_config = GolemConfig::from(config_path(TASK_TYPE)?.join("config.json"))?;
            log::info!("Using: {:#?}", golem_config);
            let workdir = WorkDir::new(TASK_TYPE)?;
            log::info!("Working directory: {}", workdir.base_dir().display());
            Ok(RunnerContext {
                engine_ref: self,
                golem_config,
                js_path: wasm_path.with_extension("js"),
                wasm_path: wasm_path.to_path_buf(),
                workdir,
            })
        }
    }
}
for_wasmtime! {
    impl BrassEngine for gwr_backend::WtEngine {
        fn context_from_path(self, _wasm_path: &Path) -> anyhow::Result<RunnerContext<Self>> {
            anyhow::bail!("brass backed supports only spwasm runtime")
        }
    }
}

pub fn run<E: super::BrassEngine>(
    engine: E,
    wasm_path: &Path,
    skip_confirmation: bool,
    args: &[String],
) -> anyhow::Result<()> {
    let mut context = engine.context_from_path(wasm_path)?;

    if !skip_confirmation && !has_user_confirmed(&wasm_path) {
        anyhow::bail!("Task creation aborted.");
    }

    context.split(args)?;
    let (computed_task, subtask_order) = context.execute()?;
    context.merge(args, computed_task, subtask_order)?;
    log::info!("Task computed!");
    Ok(())
}

impl<E: rt::Engine> RunnerContext<E> {
    fn split(&mut self, args: &[String]) -> anyhow::Result<()> {
        let output_path = self.workdir.split_output()?;
        let mut split_args = Vec::new();
        split_args.push("split".to_owned());
        split_args.push("/task_dir/".to_owned());
        split_args.extend(args.iter().cloned());

        log::debug!("split args: {:?}", split_args);

        run_local_code(
            self.engine_ref.clone(),
            &self.wasm_path,
            &output_path,
            split_args,
        )?;

        Ok(())
    }

    fn execute(&mut self) -> anyhow::Result<(ComputedTask, Vec<String>)> {
        let wasm_file = std::fs::read(&self.wasm_path)?;
        let js_file = std::fs::read(&self.js_path)?;
        let binary = GWasmBinary {
            js: js_file.as_slice(),
            wasm: wasm_file.as_slice(),
        };

        let builder = TaskBuilder::new(self.workdir.clone(), binary)
            .name(&self.golem_config.name)
            .bid(self.golem_config.bid)
            .budget(self.golem_config.budget)
            .timeout(self.golem_config.task_timeout)
            .subtask_timeout(self.golem_config.subtask_timeout);
        let (task, subtask_order) = builder.build()?;

        log::debug!("Created task: {:#?}", task);

        log::info!("Starting task computation...");
        let subtask_count = task.options().subtasks().count();
        let address_parts: Vec<&str> = self.golem_config.address.split(':').collect();
        let computed_task = compute(
            self.golem_config.data_dir.clone(),
            address_parts[0].to_owned(),
            address_parts[1].parse()?,
            self.golem_config.net.clone(),
            task,
            ProgressUpdater::new(subtask_count as u64),
        )
        .map_err(|e| log::error!("Task computation failed: {}", e))
        .unwrap();

        log::debug!("Computed task: {:#?}", computed_task);

        Ok((computed_task, subtask_order))
    }

    fn merge(
        &mut self,
        args: &[String],
        task: ComputedTask,
        subtask_order: Vec<String>,
    ) -> anyhow::Result<()> {
        let merge_path = self.workdir.merge_path()?;
        let mut output_agg = Vec::new();

        let mut id_to_subtask = HashMap::new();
        for subtask in task.subtasks {
            id_to_subtask.insert(subtask.name.clone(), subtask);
        }

        // Read subtasks in original order
        for subtask_id in subtask_order {
            let subtask = id_to_subtask.remove(&subtask_id).unwrap();
            let output_path = self.workdir.base_dir().join(&subtask.name);
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
            self.engine_ref.clone(),
            &self.wasm_path,
            merge_path.parent().unwrap(),
            merge_args,
        )?;

        Ok(())
    }
}

fn has_user_confirmed(wasm_path: &Path) -> bool {
    println!(
        "\nYou are about to create a Brass Golem task with the above parameters. \
         \nThe WASM binary to be used for this task is: {:?}.",
        wasm_path
    );

    return prompt_default("Would you like to proceed?", false);
}
