use std::path::{Path, PathBuf};
use std::fs;
use std::env;

use failure::{Error, Fail};
use std::iter::FromIterator;
use serde::de::Unexpected::Map;

use crate::executor::{Executor, exec_for};
use crate::splitter::{Splitter, split_into};
use crate::merger::Merger;
use crate::taskdef::{FromTaskDef, IntoTaskArg, IntoTaskDef, TaskDef};



pub type TaskResult<In, Out> = Vec<(In, Out)>;



#[derive(Debug, Fail)]
pub enum ApiError {
    #[fail(display = "Can't find parent")]
    NoParent,
    #[fail(display = "Expected -- separator.")]
    NoSeparator,
    #[fail(display = "No such command {}.", command)]
    NoCommand {
        command: String,
    },
    #[fail(display = "Invalid params format: {}.", message)]
    InvalidParamsFormat {
        message: String
    },
    #[fail(display = "Json conversion error: {}.", error)]
    JsonError {
        error: serde_json::error::Error
    },
}


//pub fn save_params_vec<SplitOutputType : TaskInput>(output_file: &Path, split_params: &Vec<SplitOutputType>) -> Result<(), Error> {
//    let json_params: Vec<serde_json::Value> = split_params.iter().map(TaskInput::pack_task).collect();
//    save_json(output_file, &serde_json::json!(json_params))
//}
//
//pub fn save_params<SplitOutputType : TaskInput>(output_file: &Path, split_params: &SplitOutputType) -> Result<(), Error> {
//    let json: serde_json::Value = split_params.pack_task();
//    save_json(output_file, &json)
//}

pub fn save_json(output_file: &Path, json: &serde_json::Value) -> Result<(), Error> {

    let work_dir = output_file.parent().ok_or(ApiError::NoParent)?;

    fs::write(output_file, serde_json::to_string_pretty(&json)?)?;
    Ok(())
}

pub fn load_json(params_path: &Path) -> Result<serde_json::Value, Error> {
    let content = fs::read_to_string(params_path)?;
    return Ok(serde_json::from_str(&content)?);
}

//pub fn load_params<ArgsType: TaskInput>(params_path: &Path) -> Result<ArgsType, Error> {
//    let json = load_json(params_path)?;
//    load_params_json::<ArgsType>(json)
//}
//
//pub fn load_params_json<ArgsType: TaskInput>(json: serde_json::Value) -> Result<ArgsType, Error> {
//    if !json.is_array() {
//        Err(ApiError::InvalidParamsFormat{ message: String::from("Top level array not found") })?
//    }
//
//    ArgsType::from_json(json)
//}

fn save_task_def_vec(output_file: &Path, taskdefs: &Vec<TaskDef>) -> Result<(), Error> {

    let json_params: Result<Vec<serde_json::Value>, _> = taskdefs
        .into_iter()
        .map(|taskdef| { serde_json::to_value(taskdef) })
        .collect::<Result<_, _>>();

    save_json(output_file, &serde_json::json!(json_params?))
}

fn save_task_def(output_file: &Path, taskdefs: &TaskDef) -> Result<(), Error> {
    let output_dir = output_file.parent().ok_or(ApiError::NoParent)?;

    let json = serde_json::to_value(taskdefs.into_arg(&output_dir)?)?;
    save_json(output_file, &json)
}

//pub fn load_params_vec<ArgsType: TaskInput>(params_path: &Path) -> Result<Vec<ArgsType>, Error> {
//    let json = load_json(params_path)?;
//    match json {
//        serde_json::Value::Array(json_vec) => {
//
//            let mut params = vec!();
//            for element in json_vec.into_iter() {
//                params.push(load_params_json::<ArgsType>(element)?);
//            }
//            Ok(params)
//        },
//        _ => {
//            Err(ApiError::InvalidParamsFormat{ message: String::from("Loading json: top object is not an Array.") })?
//        }
//    }
//}

fn load_task_def(taskdef_file: &Path) -> Result<TaskDef, Error> {
    unimplemented!()
}

pub fn split_step<S: Splitter<WorkItem = In>, In: IntoTaskDef>(splitter: S, args: &Vec<String>) -> Result<(), Error> {

    // TODO: check param len
    let work_dir = PathBuf::from(&args[0]);
    let split_args = &Vec::from_iter(args[1..].iter().cloned());

    println!("Split args {:?}", split_args);

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


//pub fn merge_step<MapReduceType: MapReduce>(args: &Vec<String>) -> Result<(), Error>  {
//
//    let tasks_params_path = PathBuf::from(args[0].clone());
//    let tasks_outputs_path = PathBuf::from(args[1].clone());
//
//    if args[2] != "--" {
//        return Err(ApiError::NoSeparator)?;
//    }
//
//    let input_params = load_params_vec::<MapReduceType::ExecuteInput>(&tasks_params_path)?;
//    let outputs = load_params_vec::<MapReduceType::ExecuteOutput>(&tasks_outputs_path)?;
//
//    let in_out_pack = input_params.into_iter()
//        .zip(outputs.into_iter())
//        .collect();
//
//    let original_args = Vec::from_iter(args[3..].iter().cloned());
//
//    MapReduceType::merge(&original_args, &in_out_pack);
//    Ok(())
//}



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
    else if command == "execute" {
        execute_step(executor, &args)
    }
//    else if command == "merge" {
//        merge_step::<MapReduceType>(&args)
//    }
    else {
        Err(ApiError::NoCommand{ command })?
    }
}


#[cfg(test)]
mod test {

    use crate::splitter::{SplitContext};
    use crate::blob::{Blob, Output};
    use crate::dispatcher::{split_step};
    use std::path::PathBuf;

    /// =================================== ///
    /// Test Structures

    fn splitter1(ctx: &mut dyn SplitContext) -> Vec<(u32,)> {
        return vec![(3,)]
    }

    fn splitter2(ctx: &mut dyn SplitContext) -> Vec<(u32, Output)> {
        return vec![(3, ctx.new_blob())]
    }

    /// =================================== ///
    /// Tests


    #[test]
    fn test_splitter_with_u32() {
        split_step(&splitter1, &vec![String::from("")]).unwrap();
    }

    #[test]
    fn test_splitter_with_blob() {
        split_step(&splitter2, &vec![String::from("")]).unwrap();
    }

}
