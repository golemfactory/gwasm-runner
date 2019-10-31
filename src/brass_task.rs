use {
    crate::workdir::WorkDir,
    failure::Fallible,
    gwasm_brass_api::prelude::{GWasmBinary, Options, Subtask, Task, Timeout},
    std::{
        fs::{self, OpenOptions},
        io::BufWriter,
        path::PathBuf,
        str::FromStr,
    },
};

pub struct TaskBuilder<'a> {
    binary: GWasmBinary<'a>,
    name: Option<String>,
    bid: Option<f64>,
    timeout: Option<Timeout>,
    subtask_timeout: Option<Timeout>,
    workdir: WorkDir,
}

impl<'a> TaskBuilder<'a> {
    pub fn new(workdir: WorkDir, binary: GWasmBinary<'a>) -> Self {
        Self {
            binary,
            name: None,
            bid: None,
            timeout: None,
            subtask_timeout: None,
            workdir,
        }
    }

    pub fn name<S: AsRef<str>>(mut self, name: S) -> Self {
        self.name = Some(name.as_ref().to_owned());
        self
    }

    pub fn bid(mut self, bid: f64) -> Self {
        self.bid = Some(bid);
        self
    }

    pub fn timeout(mut self, timeout: Timeout) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn subtask_timeout(mut self, subtask_timeout: Timeout) -> Self {
        self.subtask_timeout = Some(subtask_timeout);
        self
    }

    pub fn build(mut self) -> Fallible<Task> {
        let name = self.name.take().unwrap_or_else(|| "unknown".to_owned());
        let bid = self.bid.unwrap_or(1.0);
        let timeout = self.timeout.unwrap_or_else(|| {
            Timeout::from_str("00:10:00")
                .expect("could not correctly parse default task timeout value")
        });
        let subtask_timeout = self.subtask_timeout.unwrap_or_else(|| {
            Timeout::from_str("00:10:00")
                .expect("could not correctly parse default subtask timeout value")
        });

        let js_name = format!("{}.js", name);
        let wasm_name = format!("{}.wasm", name);

        let base_input_dir = self.workdir.base_dir();
        let mut options = Options::new(
            js_name,
            wasm_name,
            base_input_dir.clone(),
            base_input_dir.clone(),
            None,
        );

        // Write binaries to task input dir
        fs::write(&base_input_dir.join(&options.js_name()), self.binary.js)?;
        fs::write(&base_input_dir.join(&options.wasm_name()), self.binary.wasm)?;

        let split_dir = self.workdir.split_output()?;
        let merge_dir = self.workdir.merge_path()?;
        let tasks_path = split_dir.join("tasks.json");
        let tasks: Vec<gwasm_dispatcher::TaskDef> =
            serde_json::from_reader(OpenOptions::new().read(true).open(tasks_path)?)?;

        let mut input_agg = Vec::new();

        // Create subtask directories and definitions
        for task in tasks {
            let subtask_dir = self.workdir.new_task()?;

            // Output does not have its separate dir since Brass does not expect subdirectories
            // in its output dir
            let subtask_input_path = subtask_dir.join("in");
            std::fs::create_dir(&subtask_input_path)?;

            for blob_path in task.blobs() {
                std::fs::rename(
                    &split_dir.join(blob_path),
                    subtask_input_path.join(blob_path),
                )?;
            }

            serde_json::to_writer_pretty(
                BufWriter::new(
                    OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(subtask_input_path.join("task.json"))?,
                ),
                &task,
            )?;

            let mut subtask = Subtask::new();

            // Define which files should be copied over from the sandbox to the host after the task
            // is finished
            for output_file in task.outputs() {
                subtask
                    .output_file_paths
                    .push(PathBuf::from("/").join(output_file));
            }
            subtask
                .output_file_paths
                .push(PathBuf::from("/").join("task.json"));

            subtask.exec_args = vec![
                "exec".to_owned(),
                "/in/task.json".to_owned(),
                "/task.json".to_owned(),
            ];

            options.add_subtask(
                subtask_dir
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
                subtask,
            );

            let task = task.rebase_output("", "../");
            input_agg.push(task.rebase_to(&subtask_input_path, &merge_dir)?);
        }

        serde_json::to_writer_pretty(
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(merge_dir.join("tasks_input.json"))?,
            &input_agg,
        )?;

        Ok(Task::new(name, bid, timeout, subtask_timeout, options))
    }
}
