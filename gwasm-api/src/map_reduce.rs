use crate::executor::Executor;
use crate::splitter::Splitter;
use crate::merger::Merger;
use crate::taskdef::{FromTaskDef, IntoTaskArg, IntoTaskDef, TaskDef};



//
//pub trait MapReduce:
//Splitter<WorkItem = In> + Executor<In, Out> + Merger<In, Out>
//{
//    type In: FromTaskDef + IntoTaskDef;
//    type Out: FromTaskDef + IntoTaskDef:
//
//}
