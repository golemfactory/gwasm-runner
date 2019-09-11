use std::path::{Path, PathBuf};

use failure::{Error};

use crate::taskdef::{FromTaskArg, FromTaskDef, IntoTaskDef, TaskDef};
use crate::splitter::{SplitContext};



pub trait Merger<In: FromTaskDef, Out: FromTaskDef> {
    fn merge(self, args_vec: &Vec<String>, tasks: Vec<(In, Out)>);
}

pub(crate) fn merge_for<M: Merger<In, Out>, In: FromTaskDef, Out: FromTaskDef>(
    merger: M,
    args_vec: &Vec<String>,
    in_outs_pack: Vec<(TaskDef, TaskDef)>,
    split_dir: &Path,
    exec_dir: &Path,
) -> Result<(), Error> {

    let in_outs: Result<Vec<(In, Out)>, Error> = in_outs_pack
        .into_iter()
        .map(|(params, output)| -> Result<(In, Out), _> {
            Ok((In::from_task_def(params, split_dir)?, Out::from_task_def(output, exec_dir)?))
    }).collect();

    merger.merge(args_vec, in_outs?);
    Ok(())
}


impl<In: FromTaskDef, Out: FromTaskDef, F: FnOnce(&Vec<String>, Vec<(In, Out)>)> Merger<In, Out> for F {

    fn merge(self, args_vec: &Vec<String>, tasks: Vec<(In, Out)>) {
        self(args_vec, tasks);
    }
}

