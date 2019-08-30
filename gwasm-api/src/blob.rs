use std::path::{PathBuf, Path};

use crate::task_params::{TaskInputElem, InputDesc};



#[derive(Default)]
pub struct Blob {
    path: Option<PathBuf>,
}



impl TaskInputElem for Blob {
    fn to_input_desc() -> InputDesc {
        InputDesc::Blob
    }

    fn pack_task(&self) -> serde_json::Value {
        serde_json::json! {
            {"path": "/tmp/xxxx"}
        }
    }
}


impl Blob {

    pub fn new(path: &str) -> Blob {
        Blob{path: Option::from(Path::new(path).to_path_buf())}
    }


}
