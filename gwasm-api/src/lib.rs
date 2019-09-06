pub mod blob_old;
pub mod task_params;
pub mod dispatcher;

pub use blob_old::{Blob};
pub use task_params::{TaskResult, TaskInput, TaskInputElem, InputDesc};

use crate::blob::Output;
use crate::executor::Executor;
use crate::splitter::Splitter;
use crate::taskdef::{FromTaskDef, IntoTaskArg, IntoTaskDef, TaskDef};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod blob;
mod error;
mod taskdef;

mod executor;
mod splitter;

pub trait Merger<In, Out> {
    fn merge(self, tasks: Vec<(In, Out)>);
}

pub trait MapReduce<In: FromTaskDef + IntoTaskDef, Out: FromTaskDef + IntoTaskDef>:
    Splitter<WorkItem = In> + Executor<In, Out> + Merger<In, Out>
{
}

// pub fn run<S: Splitter, E: executor::Executor<S::WorkItem, Out>, Out: IntoTaskDef>(s: S, e: E) {
//     unimplemented!()
// }
