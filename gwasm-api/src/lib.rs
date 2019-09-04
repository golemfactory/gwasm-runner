use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::blob::Output;
use crate::taskdef::{TaskDef, IntoTaskDef, IntoTaskArg, FromTaskDef};
use crate::executor::Executor;
use crate::splitter::Splitter;

mod taskdef;
mod blob;
mod error;

mod executor;
mod splitter;


pub trait Merger<In, Out> {

    fn merge(self, tasks : Vec<(In, Out)>);

}


pub trait MapReduce<In : FromTaskDef + IntoTaskDef, Out : FromTaskDef + IntoTaskDef> :  Splitter<WorkItem=In> + Executor<In, Out> + Merger<In, Out> {

}



pub fn run<S : Splitter, E : executor::Executor<S::WorkItem, Out>, Out : IntoTaskDef>(s : S, e : E) {
    unimplemented!()
}