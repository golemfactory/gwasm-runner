use crate::error::Error;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskArg {
    Meta(serde_json::Value),
    Blob(String),
    Output(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskDef(Vec<TaskArg>);

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
