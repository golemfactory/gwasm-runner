pub mod dispatcher;

pub use crate::splitter::{SplitContext};
pub use crate::blob::{Blob, Output};
pub use crate::dispatcher::{TaskResult};

use crate::executor::Executor;
use crate::merger::Merger;
use crate::splitter::Splitter;
use crate::taskdef::{FromTaskDef, IntoTaskArg, IntoTaskDef, TaskDef};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod blob;
mod error;
mod taskdef;

mod executor;
mod merger;
mod splitter;
