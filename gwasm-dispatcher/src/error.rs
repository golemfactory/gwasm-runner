use std::{io, path};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    IO(#[source] io::Error),

    #[error("{0}")]
    Prefix(path::StripPrefixError),

    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("{0}")]
    Json(serde_json::error::Error),

    #[error("invalid arg")]
    MetaExpected,

    #[error("Expected blob entry.")]
    BlobExpected,

    #[error("Expected output entry.")]
    OutputExpected,
}

impl Error {
    pub fn invalid_path(path: &path::Path) -> Self {
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
