use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::io::{Read, Write};
use regex::Regex;
use std::mem;
use std::fmt;

use std::io::Error as io_err;
use crate::errors::io_other_err_wrapper;
use crate::constants::{MAGIC_WORD, MAX_FILE_SIZE, AllowedColTypes};
use crate::storage::string_handlers::{StrLenCheckType, read_string_from_buf};

#[cfg(test)]
#[path = "../tests/test_column_structs.rs"]
mod test_column_structs;

const COL_HEADER_MIN_SIZE: usize = 14;
const COL_HEADER_DATA_SIZE_OFFSET: u64 = 8;
const COL_HEADER_OVERFLOW_OFFSET: u64 = 7;

pub struct ColHeader
{
    magic_word: u32,    // magic word saying that this is our db file
    col_id: u16,        // equal to file number - we may have many files for 
                        // one column
    col_type: AllowedColTypes,       // either 1 - string or 0 - i64
    is_overflow: bool,  // tells us if there are more files with this col data
                        // last file in sequence will have it set to false
    size_of_data: u32,  // size of data without metadata
    col_name: String    // max 255 characters
}

impl ColHeader
{
    pub fn new(
        col_id: u16,
        col_type: AllowedColTypes,
        is_overflow: bool,
        size_of_data: u32,
        col_name: String
    ) -> Result<ColHeader, io_err>
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
        col_type: AllowedColTypes, 
        col_name: String
    ) -> Result<ColHeader, io_err>
    {
        ColHeader::new(0, col_type, false, 0, col_name)
    }

    /// Function creates next header in sequence. <br>
    /// So it increases col_id, sets overflow to false, size_of_data to 0
    /// and returns new ColHeader object
    pub fn create_next(&self) -> Result<ColHeader, io_err>
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

    pub fn save_to_file(&self, path: &str) -> Result<(String, File), io_err>
    {
        // Function returns file_name of created file and its File Handler
        // 
        // In save_to_file function we always create a new file even if it 
        // already existed, this function should be invoked only once when 
        // creating column file for the first time
        if path.len() == 0
        {
            return Err(io_other_err_wrapper("ColHeader - save_to_file - path len == 0"));
        }

        let last_path_char = path.as_bytes()[path.len() - 1];
        let file_name: String;

        if last_path_char == b'/'
        {
            file_name = format!("{}{}_{}", path, self.col_name, self.col_id);
        }
        else 
        {
            file_name = format!("{}/{}_{}", path, self.col_name, self.col_id);
        }

        let mut f = File::create(&file_name)?;

        let null_terminator = [b'\0'];
        let header_size = mem::size_of_val(&self.magic_word)
            + mem::size_of_val(&self.col_id)
            + mem::size_of_val(&AllowedColTypes::to_u8(&self.col_type))
            + mem::size_of_val(&self.is_overflow)
            + mem::size_of_val(&self.size_of_data)
            + self.col_name.len() + 1;

        // In one file we can have only u32::MAX bytes with HEADERS bytes
        if u32::MAX - (header_size as u32) < self.size_of_data
        {
            return Err(io_other_err_wrapper("ColHeader - save_to_file - size_of_data + header data size exceeds u32::MAX"));
        }

        f.write(&self.magic_word.to_be_bytes())?;
        f.write(&self.col_id.to_be_bytes())?;
        f.write(&AllowedColTypes::to_u8(&self.col_type).to_be_bytes())?;

        let is_overflow: u8 = self.is_overflow.try_into().unwrap();

        f.write(&is_overflow.to_be_bytes())?;
        f.write(&self.size_of_data.to_be_bytes())?;

        // When saving strings to files we need to add null termination '\0'
        // to the end of string, since rust uses pointer+length encoding
        f.write(&self.col_name.as_bytes())?;
        f.write(&null_terminator)?;

        f.flush()?;

        Ok((file_name, f))
    }
    
    pub fn read_from_buf(
        curr_buf_idx: &mut usize,
        bytes_read: usize,
        buf: &[u8],
    ) -> Result<ColHeader, io_err>
    {
        // TODO: add better checking if we haave enough bytes to read
        if (bytes_read - *curr_buf_idx) < COL_HEADER_MIN_SIZE
        {
            return Err(io_other_err_wrapper(&format!("To read column header we need to have buffer size at least: {}", COL_HEADER_MIN_SIZE)));
        }

        // TODO: add dynamic buff idx calculations based on variables lengths
        let magic_word = u32::from_be_bytes(
                    buf[..4]
                    .try_into()
                    .expect("ColHeader - read_from_buf - magic word from buff transformation error"));

        if magic_word != MAGIC_WORD
        {
            return Err(io_other_err_wrapper("ColHeader - read_from_buf - magic word is incorrect"));
        }

        let col_id = u16::from_be_bytes(
                            buf[4..6]
                            .try_into()
                            .unwrap());
        let col_type = match AllowedColTypes::from_u8(buf[6])
                        {
                            Ok(v) => v,
                            Err(e) => return Err(io_other_err_wrapper(&e))
                        };

        let is_overflow: bool = buf[7] == 1;

        // We allow size of data to be 0
        let size_of_data = u32::from_be_bytes(
                    buf[8..12]
                    .try_into()
                    .expect("ColHeader - read_from_buf - size_of_data transformation error"));

        *curr_buf_idx = 12;
        let mut res_str = String::new();

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
    ) -> Result<(), &str>
    {
        // When we read next data chunk we want to append this data to 
        // file, so we firstly need to update metadata about data size in this 
        // file so that we can check if we have enough space to append new data 
        // or if we need to create next file
        // --> if we need to create next file, we need to change is_overflow
        // flag in current file and also in METADATA we need to add new 
        // file_path for this column

        // For now we want to store WHOLE chunks in our files, so if one whole
        // chunk does not fit, we need to create a new file
        let new_data_size = match self.size_of_data.checked_add(new_data_len)
        {
            None => {
                self.is_overflow = true;
                return Err("new data chunk won't fit in curr file");
            },
            Some(new_size) => new_size
        };

        // If we have enough space we increase size we store in header, but 
        // currently we do it only in-memory, when we won't have enough space
        // in file, we will both append new data to file AND modify header
        self.size_of_data = new_data_size;
        println!("New data size: {}", self.size_of_data);

        Ok(())
    }

    pub fn modify_data_size_in_file(
        &self, 
        f: &mut File, 
    ) -> Result<(), io_err>
    {
        if self.is_overflow
        {
            f.seek(SeekFrom::Start(COL_HEADER_OVERFLOW_OFFSET))?;
            let x: u8 = self.is_overflow as u8;
            f.write(&x.to_be_bytes())?;
        }
        
        f.seek(SeekFrom::Start(COL_HEADER_DATA_SIZE_OFFSET))?;
        f.write(&self.size_of_data.to_be_bytes())?;

        Ok(())
    }

    // GETTERS
    pub fn col_id(&self) -> u16 {
        self.col_id
    }

    pub fn col_type(&self) -> AllowedColTypes {
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

pub struct ColData
{
    h: ColHeader,
    data: Vec<u8>
}



// TODO: we shouldnt return io_err everywhere
fn check_col_name_correctness(col_name: &String) -> Result<(), io_err>
{
    if col_name.len() > 255
    {
        return Err(io_other_err_wrapper("ColHeader - column name exceeds 255 characters"));
    }

    if !col_name.is_ascii()
    {
        return Err(io_other_err_wrapper(&format!("ColHeader - column: '{}' is not ASCII", &col_name)));
    }
    
    let re = Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$").unwrap();

    if !re.is_match(&col_name) {
        return Err(io_other_err_wrapper("Column names must match regex: ^[a-zA-Z][a-zA-Z0-9_]*$"));
    }
    Ok(())
}