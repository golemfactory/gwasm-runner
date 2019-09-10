/**
 Binary Large Objects

**/

use std::io::{self, Read, Write, Seek};
use std::path::{Path, PathBuf};
use std::fs;
use crate::taskdef::{IntoTaskArg, TaskArg, FromTaskArg};
use crate::error::Error;

pub struct Blob(PathBuf);

pub struct Output(pub (crate) PathBuf);

impl Blob {

    pub fn from_output(output : Output) -> Self {
        Blob(output.0)
    }

    pub fn open(&self) -> io::Result<impl Read + Seek> {
        fs::OpenOptions::new().read(true).open(&self.0)
    }

}

impl Output {

    pub fn open(&self) -> io::Result<impl Write + Seek> {
        fs::OpenOptions::new().create(true).truncate(true).write(true).open(&self.0)
    }

    #[inline]
    pub fn into_blob(self) -> Blob {
        Blob::from_output(self)
    }

    pub fn save_bytes(self, data : impl AsRef<[u8]>) -> Result<Blob, Error> {
        let data = data.as_ref();
        self.open()?.write_all(data)?;
        Ok(Blob::from_output(self))
    }

}

impl IntoTaskArg for Blob {
    fn into_arg(&self, base: &Path) -> Result<TaskArg, Error> {
        let path = self.0.strip_prefix(base)?;
        path.to_str()
            .ok_or_else(|| Error::invalid_path(&self.0))
            .map(|v| TaskArg::Blob(v.into()))
    }
}

impl FromTaskArg for Blob {
    fn from_arg(arg: TaskArg, base: &Path) -> Result<Self, Error> {
        match arg {
            TaskArg::Blob(path) => Ok(Blob(base.join(path))),
            _ => Err(Error::BlobExpected)
        }
    }
}

impl IntoTaskArg for Output {
    fn into_arg(&self, base: &Path) -> Result<TaskArg, Error> {
        let path = self.0.strip_prefix(base)?;
        path.to_str()
            .ok_or_else(|| Error::invalid_path(&self.0))
            .map(|v| TaskArg::Output(v.into()))
    }
}

