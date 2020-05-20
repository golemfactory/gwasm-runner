use std::{error::Error as StdErr, fmt, io, path};

pub(crate) type DynError = Box<dyn StdErr>;

#[derive(Debug)]
pub enum Error {
    //#[fail(display = "{}", _0)]
    IO(io::Error),

    //#[fail(display = "{}", _0)]
    Prefix(path::StripPrefixError),

    //#[fail(display = "invalid path: {}", _0)]
    InvalidPath(String),

    //#[fail(display = "{}", _0)]
    Json(serde_json::error::Error),

    //#[fail(display = "invalid arg")]
    MetaExpected,

    //#[fail(display = "Expected blob entry.")]
    BlobExpected,

    //#[fail(display = "Expected output entry.")]
    OutputExpected,
}

impl StdErr for Error {
    fn source(&self) -> Option<&(dyn StdErr + 'static)> {
        match self {
            Error::IO(e) => Some(e),
            Error::Prefix(e) => Some(e),
            Error::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::IO(e) => e.fmt(f),
            Self::Prefix(e) => e.fmt(f),
            Self::InvalidPath(msg) => write!(f, "invalid path: {}", msg),
            Self::Json(e) => e.fmt(f),
            Self::MetaExpected => write!(f, "invalid arg"),
            Self::BlobExpected => write!(f, "Expected blob entry."),
            Self::OutputExpected => write!(f, "Expected output entry."),
        }
    }
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
