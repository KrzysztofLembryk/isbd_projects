use std::fs::File;
use std::io::{Write};
use regex::Regex;

use crate::db::constants::{MAX_COL_NAME_LEN, MAX_STR_DATA_LEN};
use crate::db::errors::{DbError};

#[derive(PartialEq)]
pub enum StrLenCheckType
{
    ColNameLenCheck,
    DataLenCheck,
    NoCheck,
}

pub fn save_string_to_file_with_null_char(
    s: &String, 
    f: &mut File
) -> Result<(), DbError>
{
    if s.len() > MAX_COL_NAME_LEN
    {
        return Err(DbError::SizeExceeded{
            msg: format!("save_string_to_file_with_null_char:"),  
            max: MAX_COL_NAME_LEN
        }
        );
    }
    let null_terminator = [b'\0'];

    if !s.is_ascii()
    {
        return Err(DbError::UnsupportedType(
            format!("save_string_to_file_with_null_char: '{}' is not ASCII", s)));
    }

    f.write(s.as_bytes())?;
    f.write(&null_terminator)?;

    Ok(())
}

pub fn read_string_from_buf(
    curr_buf_idx: &mut usize,
    bytes_read: usize,
    buf: &[u8],
    res_str: &mut String,
    len_check_type: StrLenCheckType
) -> Result<bool, DbError>
{
    // -- Function reads from buffer characters and appends them to res_str up
    // until encountering NULL Terminator or exceeding number of characters 
    // allowed by given len_check_type.
    // -- If whole string read, it returns OK(eos_present = true), otherwise it 
    // returns Ok(eos_present = false) indicating that we don't have data left
    // in the buffer.
    // -- If we encounter non ascii character or length of read string is too 
    // big we return Err()
    let mut eos_present = false;
    loop
    {
        if *curr_buf_idx >= bytes_read
        {
            break;
        }

        let c = buf[*curr_buf_idx];
        // When saving metadata file, for every string we append null 
        // terminator
        if c == b'\0'
        {
            eos_present = true;
            *curr_buf_idx += 1;
            break;
        }

        // When saving metadata file we checked if all characters are ascii.
        // However someone may have changed our file, or give us incorrect 
        // one thus we need to check that again when reading.
        if !c.is_ascii()
        {
            return Err(DbError::InternalDbError("read_string_from_buf: we've read not an ASCII character".to_string()));
        }

        res_str.push(c as char);

        if len_check_type == StrLenCheckType::ColNameLenCheck 
            && res_str.len() > MAX_COL_NAME_LEN
        {
            return Err(DbError::SizeExceeded{
                msg: format!("read_string_from_buf - column name: '{}'", res_str),
                max: MAX_COL_NAME_LEN
            }
            );
        }
        else if len_check_type == StrLenCheckType::DataLenCheck
            && res_str.len() > MAX_STR_DATA_LEN
        {
            return Err(DbError::SizeExceeded{
                msg: format!("read_string_from_buf: VARCHAR has too big size({}), we do not allow such string size in our database", res_str.len()), 
                max: MAX_STR_DATA_LEN
            });
        }

        *curr_buf_idx += 1;
    }

    Ok(eos_present)
}


pub fn check_col_name_correctness(col_name: &String) -> Result<(), DbError>
{
    if col_name.len() > MAX_COL_NAME_LEN
    {
        return Err(DbError::InvalidColumnName {
            msg: format!("Column name exceeds maximum length {}", MAX_COL_NAME_LEN),
            name: col_name.clone()
        });
    }

    if !col_name.is_ascii()
    {
        return Err(DbError::InvalidColumnName {
            msg: "Column name contains non-ASCII characters".to_string(),
            name: col_name.clone()
        });
    }
    
    let re = Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$")
        .map_err(|e| DbError::Other(format!("Failed to compile regex: {}", e)))?;

    if !re.is_match(&col_name) {
        return Err(DbError::InvalidColumnName {
            msg: "Column name must start with a letter and contain only letters, numbers, and underscores".to_string(),
            name: col_name.clone()
        });
    }
    Ok(())
}