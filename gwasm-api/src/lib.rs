pub mod blob_old;
pub mod dispatcher;
pub mod task_params;

pub use blob_old::Blob;
pub use task_params::{InputDesc, TaskInput, TaskInputElem, TaskResult};

use crate::blob::Output;
use crate::executor::Executor;
use crate::merger::Merger;
use crate::splitter::Splitter;
use crate::taskdef::{FromTaskDef, IntoTaskArg, IntoTaskDef};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod blob;
mod error;
mod taskdef;

mod executor;
mod merger;
mod splitter;

pub use taskdef::{TaskArg, TaskDef};

pub fn run<S: Splitter, E: executor::Executor<S::WorkItem, Out>, Out: IntoTaskDef>(s: S, e: E) {
    unimplemented!()
}

pub trait MapReduce<In: FromTaskDef + IntoTaskDef, Out: FromTaskDef + IntoTaskDef>:
    Splitter<WorkItem = In> + Executor<In, Out> + Merger<In, Out>
{
}

// pub fn run<S: Splitter, E: executor::Executor<S::WorkItem, Out>, Out: IntoTaskDef>(s: S, e: E) {
//     unimplemented!()
// }
