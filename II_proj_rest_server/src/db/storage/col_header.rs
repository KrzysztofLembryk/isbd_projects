use crate::db::constants::{LogicalColType, DB_DATA_DIR, MAGIC_WORD, MAX_FILE_SIZE};
use crate::db::storage::string_handlers::{StrLenCheckType, read_string_from_buf, check_col_name_correctness};
use crate::db::errors::{DbError};

use tokio::fs as tokio_fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncSeek, SeekFrom, AsyncSeekExt};
use std::mem;
use std::fmt;

// metadata plus size plus one char for col name and null terminator
const COL_HEADER_MIN_SIZE: usize = 14; 
const COL_HEADER_OVERFLOW_OFFSET: u64 = 7;
const COL_HEADER_DATA_SIZE_OFFSET: u64 = 8;

pub struct ColHeader
{
    magic_word: u32,    // magic word saying that this is our db file
    col_id: u16,        // equal to file number - we may have many files for 
                        // one column
    col_type: LogicalColType,
    is_overflow: bool,  // OBSOLETE
    size_of_data: u32,  // size of data without metadata
    col_name: String    // max 255 characters
}

impl ColHeader
{
    pub fn new(
        col_id: u16,
        col_type: LogicalColType,
        is_overflow: bool,
        size_of_data: u32,
        col_name: String
    ) -> Result<ColHeader, DbError>
    {
        check_col_name_correctness(&col_name)?;

        Ok(ColHeader { 
            magic_word: MAGIC_WORD, 
            col_id, 
            col_type, 
            is_overflow, 
            size_of_data, 
            col_name })
    }

    pub fn new_empty(
        col_type: LogicalColType, 
        col_name: String
    ) -> Result<ColHeader, DbError>
    {
        let col_id = 0;
        let is_overflow = false;
        let size_of_data = 0;

        ColHeader::new(col_id, col_type, is_overflow, size_of_data, col_name)
    }

    /// Function creates next header in sequence. <br>
    /// So it increases col_id, sets overflow to false, size_of_data to 0
    /// and returns new ColHeader object
    pub fn create_next(&self) -> Result<ColHeader, DbError>
    {
        let is_overflow = false;
        let size_of_data = 0;

        ColHeader::new(
            self.col_id + 1, 
            self.col_type, 
            is_overflow, 
            size_of_data, 
            self.col_name.clone())
    }

    /// - Function returns file_name of created file and its File Handler
    ///     - In save_to_file function we always create a new file even if it 
    ///       already exists, this function should be invoked only once when 
    ///       creating column file for the first time
    /// - We do not encode column header data
    pub async fn save_to_file(
        &self, 
        dir_path: &str
    ) -> Result<(String, tokio_fs::File), DbError>
    {
        if dir_path.len() == 0
        {
            return Err(DbError::Other(
                "ColHeader::save_to_file - directory path is empty".to_string()
            ));
        }

        let last_path_char = dir_path.as_bytes()[dir_path.len() - 1];
        let file_name: String;

        if last_path_char == b'/'
        {
            file_name = format!("{}{}_{}", dir_path, self.col_name, self.col_id);
        }
        else 
        {
            file_name = format!("{}/{}_{}", dir_path, self.col_name, self.col_id);
        }

        let mut f = tokio_fs::File::create(&file_name).await?;

        let null_terminator = [b'\0'];
        let header_size = mem::size_of_val(&self.magic_word)
            + mem::size_of_val(&self.col_id)
            + mem::size_of_val(&self.col_type.to_u8())
            + mem::size_of_val(&self.is_overflow)
            + mem::size_of_val(&self.size_of_data)
            + self.col_name.len() + 1;

        match MAX_FILE_SIZE.checked_sub(header_size as u32) 
        {
            Some(val) => {
                if val < self.size_of_data
                {
                    return Err(DbError::SizeExceeded {
                        msg: "ColHeader::save_to_file - size_of_data + header size exceeds MAX_FILE_SIZE".to_string(),
                        max: MAX_FILE_SIZE as usize
                    });
                }
            },
            None => {
                return Err(DbError::SizeExceeded {
                    msg: "ColHeader::save_to_file - header size exceeds MAX_FILE_SIZE".to_string(),
                    max: MAX_FILE_SIZE as usize
                });
            }
        };
        
        // TODO: do it in one write
        f.write(&self.magic_word.to_be_bytes()).await?;
        f.write(&self.col_id.to_be_bytes()).await?;
        f.write(&LogicalColType::to_u8(&self.col_type).to_be_bytes()).await?;

        let is_overflow: u8 = self.is_overflow
            .try_into()
            .map_err(|_| DbError::Other(
                "ColHeader::save_to_file - failed to convert bool to u8".to_string()
            ))?;

        f.write(&is_overflow.to_be_bytes()).await?;
        f.write(&self.size_of_data.to_be_bytes()).await?;

        // When saving strings to files we need to add null termination '\0'
        // to the end of string, since rust uses pointer+length encoding
        f.write(&self.col_name.as_bytes()).await?;
        f.write(&null_terminator).await?;

        f.flush().await?;

        Ok((file_name, f))
    }
    
    pub fn create_header_from_buf(
        curr_buf_idx: &mut usize,
        bytes_read: usize,
        buf: &[u8],
    ) -> Result<ColHeader, DbError>
    {
        let remaining_bytes = match bytes_read.checked_sub(*curr_buf_idx) {
            Some(remaining) => remaining,
            None => {
                return Err(DbError::Other(
                    format!("ColHeader::create_header_from_buf - buffer index ({}) exceeds bytes read ({})", 
                        curr_buf_idx, bytes_read)
                ));
            }
        };

        if remaining_bytes < COL_HEADER_MIN_SIZE
        {
            return Err(DbError::SizeMismatch {
                msg: "ColHeader::create_header_from_buf - insufficient bytes to read header".to_string(),
                size_1: remaining_bytes,
                size_2: COL_HEADER_MIN_SIZE
            });
        }

        // TODO: add dynamic buff idx calculations based on variables lengths
        let magic_word = u32::from_be_bytes(
                    buf[..4]
                    .try_into()
                    .map_err(|_| DbError::Other(
                        "ColHeader::read_from_buf - failed to read magic word (expected 4 bytes)".to_string()
                    ))?);

        if magic_word != MAGIC_WORD
        {
            return Err(DbError::Other(
                format!("ColHeader::read_from_buf - invalid magic word: expected 0x{:X}, got 0x{:X}", 
                    MAGIC_WORD, magic_word)
            ));
        }

        let col_id = u16::from_be_bytes(
                            buf[4..6]
                            .try_into()
                            .map_err(|_| DbError::Other(
                                "ColHeader::read_from_buf - failed to read col_id (expected 2 bytes)".to_string()
                            ))?);
        let col_type = LogicalColType::from_u8(buf[6])
                            .map_err(|e| DbError::UnsupportedType(
                                format!("ColHeader::read_from_buf - {}", e)
                            ))?;

        let is_overflow: bool = buf[7] == 1;

        // We allow size of data to be 0
        let size_of_data = u32::from_be_bytes(
                    buf[8..12]
                    .try_into()
                    .map_err(|_| DbError::Other(
                        "ColHeader::read_from_buf - failed to read size_of_data (expected 4 bytes)".to_string()
                    ))?);

        *curr_buf_idx = 12;
        let mut res_str = String::new();

        // TODO: we shouldnt expect that, change it to loop
        // col name is maximally 255 characters, our buffer will have greater  
        // size than this, thus we expect to be able to read whole column name
        // in one go
        read_string_from_buf(
            curr_buf_idx, 
            bytes_read, 
            buf, 
            &mut res_str, 
            StrLenCheckType::ColNameLenCheck)?;
        check_col_name_correctness(&res_str)?;

        Ok(ColHeader { 
            magic_word, 
            col_id, 
            col_type, 
            is_overflow, 
            size_of_data, 
            col_name: res_str 
        })
    }

    pub fn increase_data_size(
        &mut self, 
        new_data_len: u32
    ) -> Result<(), usize>
    {
        // When we read next data chunk we want to append this data to 
        // file, so we firstly need to update metadata about data size in this 
        // file so that we can check if we have enough space to append new data 
        // or if we need to create next file
        // --> if we need to create next file, we need to change is_overflow
        // flag in current file and also in METADATA we need to add new 
        // file_path for this column

        let new_data_size = match self.size_of_data.checked_add(new_data_len)
        {
            None => {
                self.is_overflow = true;
                let available_free_space = MAX_FILE_SIZE - self.size_of_data;
                self.size_of_data = MAX_FILE_SIZE;

                return Err(available_free_space as usize);
            },
            Some(new_size) => {
                if new_size <= MAX_FILE_SIZE 
                {
                    new_size
                }
                else 
                {
                    self.is_overflow = true;
                    let available_free_space = MAX_FILE_SIZE - self.size_of_data;
                    self.size_of_data = MAX_FILE_SIZE;

                    return Err(available_free_space as usize);
                }
            }
        };

        self.size_of_data = new_data_size;

        Ok(())
    }

    pub async fn modify_data_size_in_file(
        &self, 
        f: &mut tokio_fs::File, 
    ) -> Result<(), DbError>
    {
        if self.is_overflow
        {
            f.seek(SeekFrom::Start(COL_HEADER_OVERFLOW_OFFSET)).await?;
            let x: u8 = self.is_overflow as u8;
            f.write(&x.to_be_bytes()).await?;
        }
        
        f.seek(SeekFrom::Start(COL_HEADER_DATA_SIZE_OFFSET)).await?;
        f.write(&self.size_of_data.to_be_bytes()).await?;

        Ok(())
    }

    // GETTERS
    pub fn col_id(&self) -> u16 {
        self.col_id
    }

    pub fn col_type(&self) -> LogicalColType {
        self.col_type
    }

    pub fn is_overflow(&self) -> bool {
        self.is_overflow
    }

    pub fn size_of_data(&self) -> u32 {
        self.size_of_data
    }

    pub fn col_name(&self) -> &String {
        &self.col_name
    }

    pub fn get_file_path(&self) -> String
    {
        format!("{}/{}_{}", DB_DATA_DIR, self.col_name ,self.col_id)
    }

    pub fn get_next_file_path(&self) -> Result<String, DbError>
    {
        if self.is_overflow
        {
            return Ok(format!("{}/{}_{}", DB_DATA_DIR, self.col_name ,self.col_id + 1));
        }

        Err(DbError::Other("There is no next file path since there is no overflow in current file".to_string()))
    }
}

impl fmt::Display for ColHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ColHeader:")?;
        writeln!(f, "  magic_word: 0x{:X}", self.magic_word)?;
        writeln!(f, "  col_id: {}", self.col_id)?;
        writeln!(f, "  col_type: {:?}", self.col_type)?;
        writeln!(f, "  is_overflow: {}", self.is_overflow)?;
        writeln!(f, "  size_of_data: {}", self.size_of_data)?;
        writeln!(f, "  col_name: {}", self.col_name)?;
        Ok(())
    }
}