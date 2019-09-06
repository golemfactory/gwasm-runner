use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use failure::{Error, Fail};


pub type TaskResult<In, Out> = Vec<(In, Out)>;


#[derive(Clone, Copy, Debug)]
pub enum InputDesc {
    Meta,
    Blob,
}

pub trait TaskInput : Sized {
    fn to_input_desc() -> Vec<InputDesc>;

    fn pack_task(&self) -> serde_json::Value;
    fn from_json(json: serde_json::Value) -> Result<Self, Error>;
}

pub trait TaskInputElem : Sized {
    fn to_input_desc() -> InputDesc;

    fn pack_task(&self) -> serde_json::Value;
    fn from_json(json: serde_json::Value) -> Result<Self, Error>;
}

impl<S: Serialize + DeserializeOwned> TaskInputElem for S {
    fn to_input_desc() -> InputDesc {
        InputDesc::Meta
    }

    fn pack_task(&self) -> serde_json::Value {
        serde_json::json! {
            { "meta": self }
        }
    }

    fn from_json(json: serde_json::Value) -> Result<Self, Error> {
        Ok(serde_json::from_value(json["meta"].clone())?)
    }
}

impl<T: TaskInputElem> TaskInput for (T,) {
    fn to_input_desc() -> Vec<InputDesc> {
        vec![T::to_input_desc()]
    }

    fn pack_task(&self) -> serde_json::Value {
        serde_json::json! {
            [self.0.pack_task()]
        }
    }

    fn from_json(json: serde_json::Value) -> Result<Self, Error> {
        Ok((T::from_json(json[0].clone())?,))
    }
}

impl<T1: TaskInputElem, T2: TaskInputElem> TaskInput for (T1, T2) {
    fn to_input_desc() -> Vec<InputDesc> {
        vec![T1::to_input_desc(), T2::to_input_desc()]
    }

    fn pack_task(&self) -> serde_json::Value {
        serde_json::json! {
            [self.0.pack_task(), self.1.pack_task()]
        }
    }

    fn from_json(json: serde_json::Value) -> Result<Self, Error> {
        Ok((T1::from_json(json[0].clone())?,
            T2::from_json(json[1].clone())?,))
    }
}

impl<T1: TaskInputElem, T2: TaskInputElem, T3: TaskInputElem> TaskInput for (T1, T2, T3) {
    fn to_input_desc() -> Vec<InputDesc> {
        vec![
            T1::to_input_desc(),
            T2::to_input_desc(),
            T3::to_input_desc(),
        ]
    }

    fn pack_task(&self) -> serde_json::Value {
        serde_json::json! {
            [self.0.pack_task(), self.1.pack_task(), self.2.pack_task()]
        }
    }

    fn from_json(json: serde_json::Value) -> Result<Self, Error> {
        Ok((T1::from_json(json[0].clone())?,
            T2::from_json(json[1].clone())?,
            T3::from_json(json[2].clone())?))
    }
}

pub fn input_desc_from_fn<T: TaskInput, F: FnOnce(()) -> Vec<T>>(_: F) -> Vec<InputDesc> {
    T::to_input_desc()
}

#[cfg(test)]
mod test {
    use crate::{input_desc_from_fn, Blob, TaskInput};

    type Args = ();

    fn produce(_: Args) -> Vec<(u64, Blob)> {
        vec![(10, Blob::default()), (11, Blob::default())]
    }

    fn map((id, f): (u64, Blob)) -> (Blob,) {
        unimplemented!()
    }

    fn reduce(_: Args, chunks: TaskResult<(u64, Blob), (Blob,)>) {
        unimplemented!()
    }

    fn produce2(_: Args) -> Vec<(String, String, (u32, u32))> {
        unimplemented!()
    }

    #[test]
    fn test_types() {
        eprintln!("prod={:?}", input_desc_from_fn(produce));
        eprintln!(
            "meta prod={:?}",
            input_desc_from_fn(|()| vec![(1, Blob::default()), (2, Blob::default())])
        );
        eprintln!("prod2={:?}", input_desc_from_fn(produce2));

        let v: Vec<serde_json::Value> = produce(()).iter().map(TaskInput::pack_task).collect();
        eprintln!("json={}", serde_json::to_string(&v).unwrap());
    }

}
