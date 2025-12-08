use serde;
use std::fmt;

const MB_1: usize = 1024 * 1024;

// ############################################################################
// ############################ BUFFER SIZES ##################################
// ############################################################################

/// Size of buffer we read data into
pub const BUF_SIZE: usize = 64 * MB_1; 
/// Number of rows we want to read in one go
pub const BATCH_SIZE: usize = 10; 

// ############################################################################
// ########################### FILES CONSTANTS ################################
// ############################################################################
// pub const FILE_PATH_REGEX: &str = r"^\.?[a-zA-Z/][a-zA-Z0-9_/.]*$";
pub const FILE_PATH_REGEX: &str = r"^\.?[a-zA-Z/]([a-zA-Z0-9_/]*|/\.)*[a-zA-Z0-9_]*$";
pub const METADATA_FILE_PATH: &str = "./db_metadata";
pub const DB_DATA_DIR: &str = "./db_data";
pub const CSV_DELIM: u8 = b';';

pub const COPY_QUERY_NAME: &str = "COPY QUERY";
pub const SELECT_QUERY_NAME: &str = "SELECT QUERY";
// ############################################################################
// ########################## MAX ALLOWED SIZES ################################
// ############################################################################
pub const NULL_TERMINATOR_SIZE: usize = 1;
pub const COLUMN_HEADER_METADATA_SIZE: usize = 12;

pub const MAX_COL_NAME_LEN: usize = 255; 
pub const MAX_COL_COUNT: usize = u16::MAX as usize;
pub const MAX_STR_DATA_LEN: usize = MB_1;
pub const MAX_FILE_SIZE: u32 = u32::MAX 
    - MAX_COL_NAME_LEN as u32 
    - NULL_TERMINATOR_SIZE as u32 
    - COLUMN_HEADER_METADATA_SIZE as u32;

pub const MAX_ALLOWED_METADATA_CHANGES: u16 = 10;
pub const MAX_DB_WORKERS: usize = 10;
// ############################################################################
// ############################ OTHER CONSTANTS ################################
// ############################################################################
pub const ZSTD_ENCODE_LEVEL: i32 = 3;
pub const MAGIC_WORD: u32 = 0xF1FAA;

use crate::db::errors::DbError;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum LogicalColType
{
    INT64 = 0,
    VARCHAR = 1
}

impl LogicalColType
{
    pub fn to_u8(&self) -> u8
    {
        match self
        {
            LogicalColType::INT64 => 0,
            LogicalColType::VARCHAR => 1,
        }
    }

    pub fn from_u8(val: u8) -> Result<LogicalColType, DbError>
    {
        if val == 0
        {
            return Ok(LogicalColType::INT64);
        }
        else if val == 1
        {
            return Ok(LogicalColType::VARCHAR);
        }
        else 
        {
            return Err(DbError::ColumnTypeMismatch("AllowedColTypese::from_u8 - got neither 0 nor 1".to_string()));
        }
    }
}

impl fmt::Display for LogicalColType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogicalColType::INT64 => write!(f, "INT64"),
            LogicalColType::VARCHAR => write!(f, "VARCHAR"),
        }
    }
}
