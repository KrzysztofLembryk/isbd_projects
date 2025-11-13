const MB_1: usize = 1024 * 1024;

pub const MAGIC_WORD: u32 = 0xF1FAA;

pub const METADATA_FILE_PATH: &str = "./db_metadata";
pub const DB_DATA_DIR: &str = "./db_data";

pub const ZSTD_ENCODE_LEVEL: i32 = 3;
pub const NULL_TERMINATOR_SIZE: usize = 1;
pub const COLUMN_HEADER_METADATA_SIZE: usize = 12;

// pub const MAX_COL_NAME_LEN: usize = 254; // without null terminator
pub const MAX_COL_NAME_LEN: usize = 50; 

// This needs to be of such size, so that BATCH_SIZE * MAX_DATA_STR_LEN
// can be stored in-memory
pub const MAX_DATA_STR_LEN: usize = 8 * MB_1;

// pub const MAX_FILE_SIZE: u32 = u32::MAX - MAX_COL_NAME_LEN as u32 - NULL_TERMINATOR_SIZE as u32 - COLUMN_HEADER_METADATA_SIZE as u32;
pub const MAX_FILE_SIZE: u32 = 100;

// buff size we read data into, needs to be at least MAX_COL_NAME_LEN bytes
// pub const CHUNK_SIZE_BYTES: usize = MAX_COL_NAME_LEN + 255; 
pub const CHUNK_SIZE_BYTES: usize = 50;

// number of rows we want to read in one go
pub const BATCH_SIZE: usize = 10; 


#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AllowedColTypes {
    IntType = 0,
    StrType = 1
}
impl AllowedColTypes
{
    pub fn from_u8(x: u8) -> Result<AllowedColTypes, String>
    {
        if x == 0
        {
            return Ok(AllowedColTypes::IntType);
        }
        else if x == 1
        {
            return Ok(AllowedColTypes::StrType);
        }
        else 
        {
            return Err(String::from("AllowedColTypese - from_u8 - got neither 0 nor 1"));
        }
    }

    pub fn to_u8(t: &AllowedColTypes) ->  u8   
    {
        *t as u8
    }
}