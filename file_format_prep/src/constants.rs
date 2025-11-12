const MB_1: usize = 1024 * 1024;

pub const MAGIC_WORD: u32 = 0xF1FAA;

pub const METADATA_FILE_PATH: &str = "./db_metadata";
pub const DB_DATA_DIR: &str = "./db_data";

pub const MAX_COL_NAME_LEN: usize = 255;
pub const MAX_DATA_STR_LEN: usize = 8 * MB_1;
// pub const MAX_FILE_SIZE: u32 = u32::MAX;
pub const MAX_FILE_SIZE: u32 = 30;

// buff size we read data into, needs to be at least MAX_COL_NAME_LEN bytes
pub const CHUNK_SIZE_BYTES: usize = MAX_COL_NAME_LEN + 255; 
pub const BATCH_SIZE: usize = 8192; // number of rows we want to read in one go


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