use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use failure::{Error, Fail};
use std::iter::FromIterator;

use crate::executor::{exec_for, Executor};
use crate::merger::{merge_for, Merger};
use crate::splitter::{split_into, Splitter};
use crate::taskdef::{FromTaskDef, IntoTaskDef, TaskDef};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::{BufReader, BufWriter};

pub type TaskResult<In, Out> = Vec<(In, Out)>;

#[derive(Debug, Fail)]
pub enum ApiError {
    #[fail(display = "Can't find parent")]
    NoParent,
    #[fail(display = "Expected -- separator.")]
    NoSeparator,
    #[fail(display = "No such command {}.", command)]
    NoCommand { command: String },
    #[fail(display = "Invalid params format: {}.", message)]
    InvalidParamsFormat { message: String },
    #[fail(display = "Json conversion error: {}.", error)]
    JsonError { error: serde_json::error::Error },
}

/// =================================== ///
/// Parameters saving/loading

fn load_from<T: DeserializeOwned>(json_file: &Path) -> Result<T, Error> {
    let inf = BufReader::new(fs::OpenOptions::new().read(true).open(json_file)?);

    Ok(serde_json::from_reader(inf)?)
}

fn save_to<T: Serialize>(output_file: &Path, value: &T) -> Result<(), Error> {
    let outf = BufWriter::new(
        fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_file)?,
    );
    serde_json::to_writer_pretty(outf, value)?;

    Ok(())
}

/// =================================== ///
/// Map/Reduce steps

pub fn split_step<S: Splitter<WorkItem = In>, In: IntoTaskDef + FromTaskDef>(
    splitter: S,
    args: &[String],
) -> Result<(), Error> {
    // TODO: check param len
    let work_dir = PathBuf::from(&args[0]);
    let split_args = &Vec::from_iter(args[1..].iter().cloned());

    let split_params = split_into(splitter, &work_dir, split_args)?;

    let split_out_path = work_dir.join("tasks.json");
    save_to(&split_out_path, &split_params)
}

pub fn execute_step<E: Executor<In, Out>, In: FromTaskDef, Out: IntoTaskDef>(
    executor: E,
    args: &[String],
) -> Result<(), Error> {
    let params_path = PathBuf::from(args[0].clone());
    let input_dir = params_path.parent().ok_or(ApiError::NoParent)?;
    let output_desc_path = PathBuf::from(args[1].clone());
    let output_dir = output_desc_path.parent().ok_or(ApiError::NoParent)?;

    let input_params = load_from(&params_path)?;
    let output_desc = exec_for(&executor, input_params, &input_dir, &output_dir)?;

    save_to(&output_desc_path, &output_desc)
}

pub fn merge_step<M: Merger<In, Out>, In: FromTaskDef, Out: FromTaskDef>(
    merger: M,
    args: &[String],
) -> Result<(), Error> {
    let tasks_params_path = PathBuf::from(args[0].clone());
    let tasks_outputs_path = PathBuf::from(args[1].clone());

    let split_work_dir = tasks_params_path.parent().ok_or(ApiError::NoParent)?;
    let exec_work_dir = tasks_outputs_path.parent().ok_or(ApiError::NoParent)?;

    if args[2] != "--" {
        return Err(ApiError::NoSeparator.into());
    }

    let input_params: Vec<TaskDef> = load_from(&tasks_params_path)?;
    let outputs: Vec<TaskDef> = load_from(&tasks_outputs_path)?;

    let in_out_pack = input_params.into_iter().zip(outputs.into_iter()).collect();

    let original_args = Vec::from_iter(args[3..].iter().cloned());

    merge_for(
        merger,
        &original_args,
        in_out_pack,
        &split_work_dir,
        &exec_work_dir,
    )
}

/// =================================== ///
/// Commands dispatcher - main run function.

pub fn run<
    S: Splitter<WorkItem = In>,
    E: Executor<S::WorkItem, Out>,
    M: Merger<In, Out>,
    Out: IntoTaskDef + FromTaskDef,
    In: IntoTaskDef + FromTaskDef,
>(
    splitter: S,
    executor: E,
    merger: M,
) -> Result<(), Error> {
    let mut args: Vec<String> = env::args().collect();
    // TODO: check param len
    let command = args[1].clone();

    // Remove program name and command.
    args.drain(0..2);

    if command == "split" {
        split_step(splitter, &args)
    } else if command == "exec" {
        execute_step(executor, &args)
    } else if command == "merge" {
        merge_step(merger, &args)
    } else {
        Err(ApiError::NoCommand { command }.into())
    }
}

/// =================================== ///
/// Tests

#[cfg(test)]
#[allow(unused)]
mod test {

    use super::{execute_step, load_from, save_to, split_step};
    use crate::blob::{Blob, Output};
    use crate::splitter::SplitContext;
    use crate::taskdef::{TaskArg, TaskDef};
    use serde_json;
    use std::fs;
    use std::path::PathBuf;

    /// =================================== ///
    /// Test Structures

    fn splitter1(ctx: &mut dyn SplitContext) -> Vec<(u32,)> {
        return vec![(3,), (5,)];
    }

    fn splitter2(ctx: &mut dyn SplitContext) -> Vec<(u32, Output)> {
        return vec![(3, ctx.new_blob())];
    }

    fn execute1(x: u32) -> (u32,) {
        (x - 2,)
    }

    fn execute2(x: u32, out: Output) -> (Blob,) {
        (Blob::from_output(out),)
    }

    /// =================================== ///
    /// Test helpers

    fn create_test_dir(name: &str) -> PathBuf {
        let test_dir = PathBuf::from("test-results/").join(name);
        fs::create_dir_all(&test_dir).unwrap();
        return test_dir;
    }

    fn remove_test_dir() {
        fs::remove_dir_all("test-results/").unwrap()
    }

    /// =================================== ///
    /// Tests

    #[test]
    fn test_splitter_with_u32() {
        let test_dir = create_test_dir("test_splitter_with_u32/");

        split_step(&splitter1, &vec![test_dir.to_str().unwrap().to_owned()]).unwrap();

        let tasks_defs_file = test_dir.clone().join("tasks.json");
        let tasks_defs: Vec<TaskDef> = load_from(&tasks_defs_file).unwrap();

        // Two subtasks
        assert_eq!(tasks_defs.len(), 2);

        // Each subtasks has one element of type Meta
        assert_eq!(tasks_defs[0].0.len(), 1);
        assert_eq!(tasks_defs[1].0.len(), 1);

        match &((tasks_defs[0]).0)[0] {
            TaskArg::Meta(x) => (),
            _ => panic!("Should be meta."),
        }

        match &((tasks_defs[1]).0)[0] {
            TaskArg::Meta(x) => (),
            _ => panic!("Should be meta."),
        }
    }

    #[test]
    fn test_splitter_with_blob() {
        let test_dir = create_test_dir("test_splitter_with_blob/");

        split_step(&splitter2, &vec![test_dir.to_str().unwrap().to_owned()]).unwrap();

        let tasks_defs_file = test_dir.clone().join("tasks.json");
        let tasks_defs = load_from::<Vec<TaskDef>>(&tasks_defs_file).unwrap();

        // One subtask two elements
        assert_eq!(tasks_defs.len(), 1);
        assert_eq!(tasks_defs[0].0.len(), 2);

        match &((tasks_defs[0]).0)[0] {
            TaskArg::Meta(x) => (),
            _ => panic!("Should be blob."),
        }

        match &((tasks_defs[0]).0)[1] {
            TaskArg::Output(x) => (),
            _ => panic!("Should be output."),
        }
    }

    #[test]
    fn test_execute_with_u32() {
        let test_dir = create_test_dir("test_execute_with_u32/");
        let out_file = test_dir
            .clone()
            .join("out1.json")
            .to_str()
            .unwrap()
            .to_owned();
        let tasks_defs_file = test_dir.clone().join("task1.json");
        let task_def = vec![TaskArg::Meta(serde_json::json!(5))];

        save_to(&tasks_defs_file, &TaskDef(task_def)).unwrap();

        execute_step(
            &execute1,
            &vec![tasks_defs_file.to_str().unwrap().to_owned(), out_file],
        )
        .unwrap();
    }
}
