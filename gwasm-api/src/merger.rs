
use crate::taskdef::{FromTaskArg, FromTaskDef, IntoTaskDef, TaskDef};
use crate::splitter::{SplitContext};



pub trait Merger<In: FromTaskDef, Out: FromTaskDef> {
    fn merge(self, args_vec: &Vec<String>, tasks: Vec<(In, Out)>);
}


impl <In: FromTaskDef, Out: FromTaskDef, F: FnOnce(&Vec<String>, Vec<(In, Out)>)> Merger<In, Out> for F {

    fn merge(self, args_vec: &Vec<String>, tasks: Vec<(In, Out)>) {
        self(args_vec, tasks);
    }
}

