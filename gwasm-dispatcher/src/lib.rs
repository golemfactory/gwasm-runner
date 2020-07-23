#![allow(clippy::needless_doctest_main)]
//! gWASM Runner API for RUST.
//!
//! ## Examples
//!
//! ```edition2018
//! use gwasm_dispatcher::{dispatcher, SplitContext};
//!
//! fn main() {
//!     dispatcher::run(
//!         move |_: &mut dyn SplitContext| {
//!             const NUM_SUBTASKS: usize = 10;
//!             let arr: Vec<u64> = (1..=100).collect();
//!             arr.chunks(NUM_SUBTASKS)
//!                 .map(|x| (x.to_vec(),))
//!                 .collect::<Vec<_>>()
//!         },
//!         |task: Vec<u64>| (task.into_iter().sum(),),
//!         |_: &Vec<String>, results: Vec<(_, _)>| {
//!             let given: u64 = results.iter().map(|(_, (result,))| result).sum();
//!             let expected: u64 = (1..=100).sum();
//!             assert_eq!(expected, given, "sums should be equal")
//!         },
//!     )
//!         .unwrap()
//! }
//! ```
//!
//!
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
