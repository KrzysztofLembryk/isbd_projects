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
    WrongSize(String),
    CompressionError(String),
    DecompressionError(String),
    UnsupportedType(String),
    NotFound(String),
    InternalDbError(String),
    CsvError(String),
    InvalidName(String),
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
            DbError::WrongSize(msg) => {
                write!(f, "WrongSize: {}", msg)
            },
            DbError::CompressionError(msg) => write!(f, "Compression error: {}", msg),
            DbError::DecompressionError(msg) => write!(f, "Decompression error: {}", msg),
            DbError::UnsupportedType(msg) => write!(f, "Unsupported type: {}", msg),
            DbError::NotFound(msg) => write!(f, "Not found: {}", msg),
            DbError::InternalDbError(msg) => write!(f, "Internal Db error: {}", msg),
            DbError::CsvError(msg) => write!(f, "CSV error: {}", msg),
            DbError::InvalidName(msg) => write!(f, "Invalid Name: '{}'", msg),
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

// for automatic IO:error conversion using ?
impl From<io_err> for DbError {
    fn from(error: io_err) -> Self {
        DbError::IoError(error)
    }
}

// for automatic csv_async:error conversion using ?
impl From<csv_async::Error> for DbError {
    fn from(error: csv_async::Error) -> Self {
        match error.kind() {
            csv_async::ErrorKind::Io(io_err) => {
                DbError::IoError(std::io::Error::new(io_err.kind(), io_err.to_string()))
            },
            _ => DbError::CsvError(error.to_string())
        }
    }
}

pub fn io_other_err_wrapper(msg: &str) -> io_err
{
    io_err::new(io_errkind::Other, msg)
}