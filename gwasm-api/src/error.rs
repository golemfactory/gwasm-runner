use failure::Fail;
use std::{io, path};
use std::path::PathBuf;

#[derive(Debug, Fail)]
pub enum Error {

    #[fail(display = "{}", _0)]
    IO(#[cause] io::Error),

    #[fail(display = "{}", _0)]
    Prefix(path::StripPrefixError),

    #[fail(display = "invalid path: {}", _0)]
    InvalidPath(String),

    #[fail(display = "{}", _0)]
    Json(serde_json::error::Error),

    #[fail(display = "invalid arg")]
    MetaExpected
}

impl Error {

    pub fn invalid_path(path : &path::Path) -> Self {
        Error::InvalidPath(path.display().to_string())
    }
}

macro_rules! map_error {
    {
        $($err: ty => $opt : ident),+
    } => {
        $(impl From<$err> for Error {
            fn from(e : $err) -> Self {
                Error::$opt(e)
            }
        })+
    };
}

map_error! {
    io::Error => IO,
    path::StripPrefixError => Prefix,
    serde_json::error::Error => Json
}