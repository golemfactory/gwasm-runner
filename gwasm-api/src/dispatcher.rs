use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use failure::{Error, Fail};
use std::iter::FromIterator;

use crate::executor::{Executor, exec_for};
use crate::splitter::{Splitter, split_into};
use crate::merger::{Merger, merge_for};
use crate::taskdef::{FromTaskDef, IntoTaskArg, IntoTaskDef, TaskDef};



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
    InvalidParamsFormat {
        message: String
    },
    #[fail(display = "Json conversion error: {}.", error)]
    JsonError {
        error: serde_json::error::Error
    },
}

/// =================================== ///
/// Parameters saving/loading

pub fn save_json(output_file: &Path, json: &serde_json::Value) -> Result<(), Error> {
    let work_dir = output_file.parent().ok_or(ApiError::NoParent)?;

    fs::write(output_file, serde_json::to_string_pretty(&json)?)?;
    Ok(())
}

pub fn load_json(params_path: &Path) -> Result<serde_json::Value, Error> {
    Ok(serde_json::from_reader(fs::OpenOptions::new().read(true).open(params_path)?)?)
}

fn save_task_def_vec(output_file: &Path, taskdefs: &Vec<TaskDef>) -> Result<(), Error> {

    let json_params: Result<Vec<serde_json::Value>, _> = taskdefs
        .into_iter()
        .map(|taskdef| { serde_json::to_value(taskdef) })
        .collect::<Result<_, _>>();

    save_json(output_file, &serde_json::json!(json_params?))
}

fn save_task_def(output_file: &Path, taskdef: &TaskDef) -> Result<(), Error> {

    let output_dir = output_file.parent().ok_or(ApiError::NoParent)?;

    let json = serde_json::to_value(taskdef)?;
    save_json(output_file, &json)
}

fn load_task_def(taskdef_file: &Path) -> Result<TaskDef, Error> {
    let json = load_json(taskdef_file)?;
    Ok(serde_json::from_value::<TaskDef>(json)?)
}

fn load_task_def_vec(taskdef_file: &Path) -> Result<Vec<TaskDef>, Error> {
    let content = fs::read_to_string(taskdef_file)?;
    Ok(serde_json::from_str::<Vec<TaskDef>>(&content)?)
}

/// =================================== ///
/// Map/Reduce steps

pub fn split_step<S: Splitter<WorkItem = In>, In: IntoTaskDef + FromTaskDef>(splitter: S, args: &Vec<String>) -> Result<(), Error> {

    // TODO: check param len
    let work_dir = PathBuf::from(&args[0]);
    let split_args = &Vec::from_iter(args[1..].iter().cloned());

    let split_params = split_into(splitter, &work_dir, split_args)?;

    let split_out_path = work_dir.join("tasks.json");
    save_task_def_vec(&split_out_path, &split_params)
}

pub fn execute_step<E: Executor<In, Out>, In: FromTaskDef, Out: IntoTaskDef >(executor: E, args: &Vec<String>) -> Result<(), Error>  {

    let params_path = PathBuf::from(args[0].clone());
    let output_desc_path = PathBuf::from(args[1].clone());
    let output_dir = output_desc_path.parent().ok_or(ApiError::NoParent)?;

    let input_params = load_task_def(&params_path)?;
    let output_desc = exec_for(&executor, input_params, &output_dir)?;

    save_task_def(&output_desc_path, &output_desc)
}

pub fn merge_step<M: Merger<In, Out>, In: FromTaskDef, Out: FromTaskDef>(merger: M, args: &Vec<String>) -> Result<(), Error>  {

    let tasks_params_path = PathBuf::from(args[0].clone());
    let tasks_outputs_path = PathBuf::from(args[1].clone());

    let split_work_dir = tasks_params_path.parent().ok_or(ApiError::NoParent)?;
    let exec_work_dir = tasks_outputs_path.parent().ok_or(ApiError::NoParent)?;

    if args[2] != "--" {
        return Err(ApiError::NoSeparator)?;
    }

    let input_params = load_task_def_vec(&tasks_params_path)?;
    let outputs = load_task_def_vec(&tasks_outputs_path)?;

    let in_out_pack = input_params.into_iter()
        .zip(outputs.into_iter())
        .collect();

    let original_args = Vec::from_iter(args[3..].iter().cloned());

    merge_for(merger, &original_args, in_out_pack, &split_work_dir, &exec_work_dir)
}

/// =================================== ///
/// Commands dispatcher - main run function.

pub fn run<S: Splitter<WorkItem = In>,
           E: Executor<S::WorkItem, Out>,
           M: Merger<In, Out>,
           Out: IntoTaskDef + FromTaskDef,
           In: IntoTaskDef + FromTaskDef>(splitter: S, executor: E, merger: M) -> Result<(), Error> {

    let mut args: Vec<String> = env::args().collect();
    // TODO: check param len
    let command = args[1].clone();

    // Remove program name and command.
    args.drain(0..2);

    if command == "split" {
        split_step(splitter, &args)
    }
    else if command == "exec" {
        execute_step(executor, &args)
    }
    else if command == "merge" {
        merge_step(merger, &args)
    }
    else {
        Err(ApiError::NoCommand{ command })?
    }
}


/// =================================== ///
/// Tests

#[cfg(test)]
mod test {

    use crate::splitter::{SplitContext};
    use crate::blob::{Blob, Output};
    use crate::dispatcher::{split_step, load_task_def_vec, execute_step, save_task_def};
    use crate::taskdef::{TaskArg, TaskDef};
    use std::path::PathBuf;
    use std::fs;
    use serde_json;


    /// =================================== ///
    /// Test Structures

    fn splitter1(ctx: &mut dyn SplitContext) -> Vec<(u32,)> {
        return vec![(3,), (5,)]
    }

    fn splitter2(ctx: &mut dyn SplitContext) -> Vec<(u32, Output)> {
        return vec![(3, ctx.new_blob())]
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
        let tasks_defs = load_task_def_vec(&tasks_defs_file).unwrap();

        // Two subtasks
        assert_eq!(tasks_defs.len(), 2);

        // Each subtasks has one element of type Meta
        assert_eq!(tasks_defs[0].0.len(), 1);
        assert_eq!(tasks_defs[1].0.len(), 1);

        match &((tasks_defs[0]).0)[0] {
            TaskArg::Meta(x) => (),
            _ => panic!("Should be meta.")
        }

        match &((tasks_defs[1]).0)[0] {
            TaskArg::Meta(x) => (),
            _ => panic!("Should be meta.")
        }
    }

    #[test]
    fn test_splitter_with_blob() {
        let test_dir = create_test_dir("test_splitter_with_blob/");

        split_step(&splitter2, &vec![test_dir.to_str().unwrap().to_owned()]).unwrap();

        let tasks_defs_file = test_dir.clone().join("tasks.json");
        let tasks_defs = load_task_def_vec(&tasks_defs_file).unwrap();

        // One subtask two elements
        assert_eq!(tasks_defs.len(), 1);
        assert_eq!(tasks_defs[0].0.len(), 2);

        match &((tasks_defs[0]).0)[0] {
            TaskArg::Meta(x) => (),
            _ => panic!("Should be blob.")
        }

        match &((tasks_defs[0]).0)[1] {
            TaskArg::Output(x) => (),
            _ => panic!("Should be output.")
        }
    }

    #[test]
    fn test_execute_with_u32() {
        let test_dir = create_test_dir("test_execute_with_u32/");
        let out_file = test_dir.clone().join("out1.json").to_str().unwrap().to_owned();
        let tasks_defs_file = test_dir.clone().join("task1.json");
        let task_def = vec![TaskArg::Meta(serde_json::json!(5))];

        save_task_def(&tasks_defs_file, &TaskDef(task_def));

        execute_step(&execute1, &vec![tasks_defs_file.to_str().unwrap().to_owned(), out_file]).unwrap();
    }

}
