use crate::error::Error;
use crate::taskdef::{IntoTaskArg, TaskArg};
use std::fs;
/**
 Binary Large Objects

**/
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};

pub struct Blob(PathBuf);

pub struct Output(pub(crate) PathBuf);

impl Blob {
    pub fn from_output(output: Output) -> Self {
        Blob(output.0)
    }

    pub fn open(&self) -> io::Result<impl Read + Seek> {
        fs::OpenOptions::new().read(true).open(&self.0)
    }
}

impl Output {
    pub fn open(&self) -> io::Result<impl Write + Seek> {
        fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&self.0)
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

impl IntoTaskArg for Output {
    fn into_arg(&self, base: &Path) -> Result<TaskArg, Error> {
        let path = self.0.strip_prefix(base)?;
        path.to_str()
            .ok_or_else(|| Error::invalid_path(&self.0))
            .map(|v| TaskArg::Output(v.into()))
    }
}
