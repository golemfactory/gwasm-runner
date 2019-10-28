use crate::error::Error;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[doc(hidden)]
pub enum TaskArg {
    Meta(serde_json::Value),
    Blob(String),
    Output(String),
}

impl TaskArg {
    fn rebase_to(&mut self, to_path: &str) -> Result<(), Error> {
        match self {
            TaskArg::Output(ref mut path) => {
                *path = format!("{}/{}", to_path, path);
            }
            TaskArg::Blob(ref mut path) => {
                *path = format!("{}/{}", to_path, path);
            }
            _ => (),
        }
        Ok(())
    }
}

#[doc(hidden)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskDef(pub Vec<TaskArg>);

impl TaskDef {
    pub fn blobs(&self) -> impl IntoIterator<Item = &str> {
        self.0.iter().filter_map(move |b| {
            if let TaskArg::Blob(path) = b {
                Some(path.as_ref())
            } else {
                None
            }
        })
    }

    pub fn outputs(&self) -> impl IntoIterator<Item = &str> {
        self.0.iter().filter_map(move |b| {
            if let TaskArg::Output(path) = b {
                Some(path.as_ref())
            } else {
                None
            }
        })
    }

    pub fn rebase_output(mut self, from_base: &str, to_base: &str) -> Self {
        for arg in &mut self.0 {
            if let TaskArg::Output(ref mut output_path) = arg {
                let blob_rel_path = if output_path.starts_with(from_base) {
                    &output_path[from_base.len()..]
                } else {
                    &output_path[..]
                };
                let new_output = format!("{}{}", to_base, blob_rel_path);
                *output_path = new_output
            }
        }
        self
    }

    pub fn rebase_to(mut self, from_base: &Path, to_path: &Path) -> Result<Self, Error> {
        let prefix = calc_rebase(from_base, to_path)
            .display()
            .to_string()
            .replace("\\", "/");
        for task_arg in &mut self.0 {
            task_arg.rebase_to(&prefix)?;
        }
        Ok(self)
    }
}

pub trait IntoTaskArg {
    fn into_arg(&self, base: &Path) -> Result<TaskArg, Error>;
}

impl<T> IntoTaskArg for T
where
    for<'a> &'a T: Serialize,
{
    fn into_arg(&self, _base: &Path) -> Result<TaskArg, Error> {
        Ok(TaskArg::Meta(serde_json::to_value(self)?))
    }
}

pub trait FromTaskArg: Sized {
    fn from_arg(arg: TaskArg, base: &Path) -> Result<Self, Error>;
}

impl<T: DeserializeOwned + Sized> FromTaskArg for T {
    fn from_arg(arg: TaskArg, _base: &Path) -> Result<Self, Error> {
        Ok(match arg {
            TaskArg::Meta(m) => serde_json::from_value(m)?,
            _ => return Err(Error::MetaExpected),
        })
    }
}

pub trait IntoTaskDef {
    fn into_task_def(&self, base: &Path) -> Result<TaskDef, Error>;
}

pub trait FromTaskDef: Sized {
    fn from_task_def(task: TaskDef, base: &Path) -> Result<Self, Error>;
}

macro_rules! gen_bind {
($($t : ident = $e : ident),+) => {
        impl<$($t : IntoTaskArg),+> IntoTaskDef for ($($t,)+) {
            fn into_task_def(&self, base : &Path) -> Result<TaskDef, Error> {
                let ($($e,)+) = self;
                Ok(TaskDef(vec![$($e.into_arg(base)?),+]))
            }
        }

        impl<$($t : FromTaskArg),+> FromTaskDef for ($($t,)+) {
            fn from_task_def(task : TaskDef, base : &Path) -> Result<Self, Error> {
                let mut task_iter = task.0.into_iter();

                Ok(($($t::from_arg(task_iter.next().unwrap(), base)?,)+))
            }
        }

    }
}

gen_bind! {T1=_0}
gen_bind! {T1=_0,T2=_1}
gen_bind! {T1=_0,T2=_1,T3=_2}
gen_bind! {T1=_0,T2=_1,T3=_2,T4=_3}
gen_bind! {T0=_0,T1=_1,T2=_2,T3=_3,T4=_4}

fn calc_rebase(from_path: &Path, to_path: &Path) -> PathBuf {
    let mut next_path = PathBuf::new();

    let mut it_from = from_path.components().peekable();
    let mut it_to = to_path.components().peekable();

    loop {
        match (it_from.peek(), it_to.peek()) {
            (Some(v1), Some(v2)) if v1 == v2 => {
                let _ = (it_from.next(), it_to.next());
            }
            _ => break,
        }
    }

    while let Some(_) = it_to.next() {
        next_path.push("..");
    }
    while let Some(c) = it_from.next() {
        next_path.push(c.as_os_str())
    }

    next_path
}

#[cfg(test)]
mod test {
    use crate::taskdef::calc_rebase;
    use std::path::PathBuf;

    #[test]
    fn test_find_base() {
        assert_eq!(
            calc_rebase("task/in".as_ref(), "merge".as_ref()),
            PathBuf::from("../task/in")
        );
        assert_eq!(
            calc_rebase("task/in".as_ref(), "task".as_ref()),
            PathBuf::from("in")
        );
        assert_eq!(
            calc_rebase("a/b/c/d".as_ref(), "a".as_ref()),
            PathBuf::from("b/c/d")
        );
        assert_eq!(
            calc_rebase("a".as_ref(), "a/b/c/d".as_ref()),
            PathBuf::from("../../..")
        );
    }
}
