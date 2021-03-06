use crate::error::Error;
use crate::taskdef::{FromTaskArg, FromTaskDef, IntoTaskDef, TaskDef};
use std::path::Path;

pub trait Executor<In: FromTaskDef, Out: IntoTaskDef> {
    fn exec(&self, task: In) -> Out;
}

pub(crate) fn exec_for<In: FromTaskDef, Out: IntoTaskDef, E: Executor<In, Out>>(
    executor: &E,
    task_input: TaskDef,
    task_input_dir: &Path,
    task_output_dir: &Path,
) -> Result<TaskDef, Error> {
    let in_dir_str = task_input_dir.display().to_string();
    let out_dir_str = format!("{}/", task_output_dir.display());

    let input = In::from_task_def(
        task_input.rebase_output(&in_dir_str, &out_dir_str),
        task_input_dir,
    )?;
    executor.exec(input).into_task_def(task_output_dir)
}

macro_rules! gen_bind {
    (
        $($t : ident = $e : ident),+
    ) => {
           impl<$($t : FromTaskArg,)+  Out : IntoTaskDef, F : Fn($($t),+) -> Out> Executor<($($t,)+), Out> for F {
                fn exec(&self, task: ($($t,)+)) -> Out {
                    let ($($e,)+) = task;
                    self($($e),+)
                }
            }
    };
}

gen_bind! {T0=_0}
gen_bind! {T0=_0,T1=_1}
gen_bind! {T0=_0,T1=_1,T2=_2}
gen_bind! {T0=_0,T1=_1,T2=_2,T3=_3}
gen_bind! {T0=_0,T1=_1,T2=_2,T3=_3,T4=_4}

#[cfg(test)]
mod test {
    use super::*;
    use crate::taskdef::TaskDef;
    use std::path::PathBuf;

    fn inc_v(v: u32) -> (u32,) {
        (v + 1,)
    }

    fn add_me(v1: u32, v2: u32) -> (u32,) {
        (v1 + v2,)
    }

    #[test]
    fn test_exec() {
        let task: TaskDef = serde_json::from_str(r#"[{"meta": 10}]"#).unwrap();

        let (v,) = Executor::exec(&inc_v, (0u32,));

        assert_eq!(v, 1);
        let ret = exec_for(&inc_v, task, &PathBuf::from("."), ".".as_ref()).unwrap();
        eprintln!("{}", serde_json::to_string(&ret).unwrap());

        let task: TaskDef = serde_json::from_str(r#"[{"meta": 10},{"meta": 15}]"#).unwrap();
        let ret = exec_for(&add_me, task, &PathBuf::from("."), ".".as_ref()).unwrap();
        eprintln!("{}", serde_json::to_string(&ret).unwrap());
    }
}
