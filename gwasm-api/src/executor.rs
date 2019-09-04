use crate::taskdef::{FromTaskDef, FromTaskArg, IntoTaskDef, TaskDef};
use std::path::{PathBuf, Path};
use crate::error::Error;

pub trait Executor<In : FromTaskDef, Out : IntoTaskDef> {

    fn exec(&self, task : In) -> Out;

}

pub (crate) fn exec_for<In : FromTaskDef, Out : IntoTaskDef, E : Executor<In, Out>>(executor : &E, task_input : TaskDef, base_dir : &Path) -> Result<TaskDef, Error> {
    let input = In::from_task_def(task_input, base_dir)?;
    executor.exec(input).into_task_def(base_dir)
}

macro_rules! gen_bind {
    (
        $($t:ident = $e : ident),+
    ) => {
           impl<$($t : FromTaskArg,)+  Out : IntoTaskDef, F : Fn($($t),+) -> Out> Executor<($($t,)+), Out> for F {
                fn exec(&self, task: ($($t,)+)) -> Out {
                    let ($($e,)+) = task;
                    self($($e),+)
                }
            }
    };
}

gen_bind!{T0=_0}
gen_bind!{T0=_0,T1=_1}
gen_bind!{T0=_0,T1=_1,T2=_2}
gen_bind!{T0=_0,T1=_1,T2=_2,T3=_3}
gen_bind!{T0=_0,T1=_1,T2=_2,T3=_3,T4=_4}


#[cfg(test)]
mod test {
    use super::*;
    use crate::taskdef::{TaskDef, TaskArg};

    fn inc_v(v : u32) -> (u32,) {
        (v+1,)
    }

    fn add_me(v1 : u32, v2 : u32) -> (u32,) {
        (v1+v2,)
    }


    #[test]
    fn test_exec() {
        let task : TaskDef = serde_json::from_str(r#"[{"meta": 10}]"#).unwrap();

        let (v,) = Executor::exec(&inc_v, (0u32,));

        assert_eq!(v, 1);
        let ret = exec_for(&inc_v, task, &PathBuf::from(".")).unwrap();
        eprintln!("{}", serde_json::to_string(&ret).unwrap());

        let task : TaskDef = serde_json::from_str(r#"[{"meta": 10},{"meta": 15}]"#).unwrap();
        let ret = exec_for(&add_me, task, &PathBuf::from(".")).unwrap();
        eprintln!("{}", serde_json::to_string(&ret).unwrap());

    }


}
