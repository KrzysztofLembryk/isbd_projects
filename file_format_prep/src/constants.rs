const MB_1: usize = 1024 * 1024;

pub const MAGIC_WORD: u32 = 0xF1FAA;

pub const METADATA_FILE_PATH: &str = "./db_metadata";
pub const DB_DATA_DIR: &str = "./db_data";

pub const MAX_COL_NAME_LEN: usize = 255;
pub const MAX_DATA_STR_LEN: usize = 8 * MB_1;
pub const MAX_FILE_SIZE: u32 = u32::MAX;