pub mod task_params;
pub mod map_reduce;
pub mod dispatcher;

pub use task_params::{TaskResult, TaskInput, TaskInputElem, InputDesc};

pub use crate::splitter::{SplitContext};
pub use crate::blob::{Blob, Output};

use crate::executor::Executor;
use crate::splitter::Splitter;
use crate::merger::Merger;
use crate::taskdef::{FromTaskDef, IntoTaskArg, IntoTaskDef, TaskDef};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod blob;
mod error;
mod taskdef;

mod executor;
mod splitter;
mod merger;
