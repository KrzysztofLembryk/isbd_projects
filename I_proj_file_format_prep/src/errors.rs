use std::io::Error as io_err;
use std::io::ErrorKind as io_errkind;
use std::fmt;

// TODO: ADD LOGGING CRATE and ERROR CRATE

#[derive(Debug)]
pub enum DbError {
    IoError(io_err),
    InvalidColumnName{ msg: String, name: String },
    ColumnTypeMismatch(String),
    SizeExceeded { msg: String, max: usize},
    SizeMismatch {  msg: String, size_1: usize, size_2: usize },
    CompressionError(String),
    DecompressionError(String),
    UnsupportedType(String),
    Other(String),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::IoError(e) => write!(f, "IO error: {}", e),
            DbError::InvalidColumnName{msg, name} => write!(f, "{}. Invalid column name: {}", msg, name),
            DbError::ColumnTypeMismatch(msg) => write!(f, "Column type mismatch: {}", msg),
            DbError::SizeExceeded { msg, max} => {
                write!(f, "{}. Size exceeded: max={}, ", msg, max)
            },
            DbError::SizeMismatch { msg, size_1, size_2 } => {
                write!(f, "{}. Size mismatch: size1={}, size2={}", msg, size_1, size_2)
            },
            DbError::CompressionError(msg) => write!(f, "Compression error: {}", msg),
            DbError::DecompressionError(msg) => write!(f, "Decompression error: {}", msg),
            DbError::UnsupportedType(msg) => write!(f, "Unsupported type: {}", msg),
            DbError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DbError::IoError(e) => Some(e),
            _ => None,
        }
    }
}

//  for automatic conversion using ?
impl From<io_err> for DbError {
    fn from(error: io_err) -> Self {
        DbError::IoError(error)
    }
}

pub fn io_other_err_wrapper(msg: &str) -> io_err
{
    io_err::new(io_errkind::Other, msg)
}