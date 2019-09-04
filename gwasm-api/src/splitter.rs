use crate::blob::Output;
use crate::taskdef::{IntoTaskDef, TaskDef};
use crate::error::Error;
use std::path::{Path, PathBuf};

pub trait SplitContext {

    fn new_blob(&mut self) -> Output;

    fn args(&self) -> &Vec<String>;

}

pub trait Splitter {
    type WorkItem : IntoTaskDef;

    fn split(self, context :&mut SplitContext) -> Vec<Self::WorkItem>;

}

impl<It : IntoIterator, F: FnOnce(&mut dyn SplitContext) -> It> Splitter for F
where  It::Item : IntoTaskDef
{
    type WorkItem = It::Item;

    fn split(self, context: &mut dyn SplitContext) -> Vec<Self::WorkItem> {
        self(context).into_iter().collect()
    }
}

struct WorkDirCtx {
    id : u64,
    work_dir : PathBuf,
    args : Vec<String>
}

impl SplitContext for WorkDirCtx {
    fn new_blob(&mut self) -> Output {
        loop {
            let id = self.id;
            self.id +=1000;
            let name = format!("{:06x}.bin", id);
            let output_path = self.work_dir.join(name);
            if !output_path.exists() {
                return Output(output_path)
            }
        }
    }

    fn args(&self) -> &Vec<String> {
        &self.args
    }
}

pub (crate) fn split_into<S : Splitter>(splitter : S, base_path : &Path) -> Result<Vec<TaskDef>, Error> {
    let mut ctx = WorkDirCtx {
        id: 1000,
        work_dir: base_path.into(),
        args: vec![]
    };
    splitter
        .split(&mut ctx).into_iter()
        .map(|item| IntoTaskDef::into_task_def(&item, base_path))
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::blob::Blob;
    use std::io::{Write};

    fn my_spliter(ctx : &mut dyn SplitContext) -> Vec<(Blob,u32)> {
        let mut out = Vec::new();
        for i in 1..10 {
            let output = ctx.new_blob();
            {
                let mut w = output.open().unwrap();
                let _ = w.write("smok smok".as_ref()).unwrap();
            }
            out.push((Blob::from_output(output),i))
        }
        out
    }

    #[test]
    fn test_split() {
        let tasks = split_into(my_spliter, &PathBuf::from("/tmp")).unwrap();

        eprintln!("{}", serde_json::to_string(&tasks).unwrap());
    }


}
