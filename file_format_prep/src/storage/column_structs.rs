use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::io::{Write};
use regex::Regex;
use std::mem;
use std::fmt;

use std::io::Error as io_err;
use crate::errors::io_other_err_wrapper;
use crate::constants::{AllowedColTypes, BATCH_SIZE, CHUNK_SIZE_BYTES, DB_DATA_DIR, MAGIC_WORD, MAX_FILE_SIZE};
use crate::storage::string_handlers::{StrLenCheckType, read_string_from_buf};
use crate::storage::encoders::{delta_encode, vle_encode_i, vle_encode_u, vle_decode_i, vle_decode_u};
use crate::storage::metadata_structs::DbMetadata;

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
        let col_id = 0;
        let is_overflow = false;
        let size_of_data = 0;

        ColHeader::new(col_id, col_type, is_overflow, size_of_data, col_name)
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

    /// - Function returns file_name of created file and its File Handler
    ///     - In save_to_file function we always create a new file even if it 
    ///       already exists, this function should be invoked only once when 
    ///       creating column file for the first time
    /// - We do not encode column header data
    pub fn save_to_file(&self, path: &str) -> Result<(String, File), io_err>
    {
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

        // In one file we can have only MAX_FILE_SIZE bytes with HEADERS bytes
        if MAX_FILE_SIZE - (header_size as u32) < self.size_of_data
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

pub trait ColType 
{
    fn col_type() -> AllowedColTypes;
}

impl ColType for i64
{
    fn col_type() -> AllowedColTypes {
        AllowedColTypes::IntType
    }
}

impl ColType for String
{
    fn col_type() -> AllowedColTypes {
        AllowedColTypes::StrType 
    }
}

pub struct ColData<T: ColType>
{
    header: ColHeader,
    data: Vec<T>
}

impl<T: ColType> ColData<T>
{
    pub fn new(header: ColHeader) -> Result<ColData<T>, String>
    {
        if header.col_type() != T::col_type()
        {
            return Err(format!("Column type mismatch btwn header and data type"));
        }

        Ok(ColData {
            header: header,
            data: Vec::new()
        })
    }
}

impl ColData<i64>
{
    pub fn is_full(&self) -> bool
    {
        self.data.len() >= BATCH_SIZE
    }

    pub fn get_mut_header(&mut self) -> &mut ColHeader
    {
        &mut self.header
    }

    pub fn push(&mut self, val: i64) -> Result<(), &str>
    {
        if self.is_full()
        {
            return Err("Data stored will be greater than batch size");
        }
        self.header.increase_data_size(std::mem::size_of::<i64>() as u32)?;
        self.data.push(val);
        Ok(())
    }

    fn vle_encode(&self) -> Vec<u8>
    {
        let delta_encoded_vec = delta_encode(&self.data);
        let mut vle_encoded_vec: Vec<u8> = Vec::new();


        // First value in vec is minimum, and can be negative, but 
        // following values are differences btwn minimum and other values
        // thus they are non-negative so we can safely cast them to u64
        for (idx, val) in delta_encoded_vec.iter().enumerate()
        {
            if idx == 0
            {
                vle_encode_i(&mut vle_encoded_vec, *val);
            }
            else 
            {
                vle_encode_u(&mut vle_encoded_vec, *val as u64);
            }
        }

        vle_encoded_vec
    }

    pub fn read_from_file(file_path: &str) -> ColData<i64>
    {
        // TODO: We should get this as argument probably
        // let file_path = format!("{}/{}_{}", 
        //                             DB_DATA_DIR, 
        //                             self.header.col_name(), 
        //                             self.header.col_id()
        // );

        let mut buf = [0 as u8; CHUNK_SIZE_BYTES];

        let mut f = File::open(file_path).unwrap();
        let mut bytes_read;
        let mut buf_idx = 0;

        bytes_read = f.read(&mut buf).unwrap();

        let header = ColHeader::read_from_buf(&mut buf_idx, bytes_read, &buf).unwrap();
        let mut bytes: Vec<u8> = Vec::new();
        let mut first_value = true;
        let mut result_vec: Vec<i64> = Vec::new();
        let mut min_val: i64 = 0;

        loop 
        {
            if buf_idx >= bytes_read
            {
                buf_idx = 0;
                bytes_read = f.read(&mut buf).unwrap();

                if bytes_read == 0 {break;}
            }

            let byte = buf.get(buf_idx).unwrap();
            bytes.push(*byte);

            // if most significant bit is 0 it means that this is last byte
            // in vle encoded sequence, so we need to decode it into i64
            if byte & 0x80 == 0
            {
                // only first value might be negative
                if first_value
                {
                    let decoded_val = vle_decode_i(&bytes);

                    bytes.clear();
                    result_vec.push(decoded_val);

                    min_val = decoded_val;
                    first_value = false;
                }
                else 
                {
                    let decoded_val = vle_decode_u(&bytes);
                    println!("decoded val: {}", decoded_val);
                    bytes.clear();
                    result_vec.push(min_val + (decoded_val as i64));

                }
            }
            buf_idx += 1;
        }

        ColData {
            header: header,
            data: result_vec
        }
    }

    pub fn create_and_save_to_file(&mut self) -> (String, File)
    {
        let encoded_vec = self.vle_encode();
        let (file_name, mut f) = self.header.save_to_file(DB_DATA_DIR).unwrap();

        f.write_all(&encoded_vec).unwrap();

        (file_name, f)
    }

    pub fn append_to_file(&mut self, mut f: File)
    {
        let encoded_vec = self.vle_encode();

        f.write_all(&encoded_vec).unwrap();
    }

    /// This function should be used when we initialize database and create all
    /// files and need to count bytes etc
    fn save_data_chunk_to_file(
        &mut self,
        mut f: File,            
        bytes_read: usize,
        buf: &[u8; CHUNK_SIZE_BYTES],
        db_meta: &mut DbMetadata
    ) ->Result<File, io_err>
    {
        match self.header.increase_data_size(bytes_read as u32)
        {
            Ok(_) => {
                // We will append to a file so we always know were to write
                f.seek(SeekFrom::End(0))?;
                f.write(&buf[..bytes_read])?;
                return Ok(f);
            }
            Err(e) => {
                println!("save_data_chunk_to_file: {e}");

                // not enough free space in file, thus we need to create a new
                // file, but before that we save updated col_header to a file
                self.header.modify_data_size_in_file(&mut f)?;

                // We no longer need old col_header, we will write to a new file
                self.header = self.header.create_next()?;
                let (file_path, new_f) = self.header.save_to_file(DB_DATA_DIR)?;

                // Now we need to update our metadata
                db_meta
                    .append_new_file_path(self.header.col_name(), file_path)?;

                // And now we recursively invoke this function, since now we 
                // will go into OK branch
                return self.save_data_chunk_to_file(
                    new_f, 
                    bytes_read, 
                    buf,
                    db_meta);
            }
        }
    }


    pub fn data(&self) -> &Vec<i64>
    {
        &self.data
    }
}

//##############################################################################
//######################## PRIVATE HELPER FUNCTIONS ############################
//##############################################################################
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

