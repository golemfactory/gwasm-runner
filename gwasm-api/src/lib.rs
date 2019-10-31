pub mod dispatcher;

pub use crate::blob::{Blob, Output};
pub use crate::dispatcher::TaskResult;
pub use crate::splitter::SplitContext;

mod blob;
mod error;
mod taskdef;

mod executor;
mod merger;
mod splitter;

pub use taskdef::{TaskArg, TaskDef};
