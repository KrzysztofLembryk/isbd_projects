use std::io::Error as io_err;
use crate::constants::{MAX_COL_NAME_LEN, MAX_DATA_STR_LEN};
use crate::errors::io_other_err_wrapper;

#[derive(PartialEq)]
pub enum StrLenCheckType
{
    ColNameLenCheck,
    DataLenCheck,
    NoCheck,
}

pub fn read_string_from_buf(
    curr_buf_idx: &mut usize,
    bytes_read: usize,
    buf: &[u8],
    res_str: &mut String,
    len_check_type: StrLenCheckType
) -> Result<bool, io_err>
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
            return Err(io_other_err_wrapper("DbMetadata - read_metadata - we've read not an ASCII character"));
        }

        res_str.push(c as char);

        if len_check_type == StrLenCheckType::ColNameLenCheck 
            && res_str.len() > MAX_COL_NAME_LEN
        {
            return Err(io_other_err_wrapper(&format!("read_string_from_buf - column name exceeds {} characters, name: {}", MAX_COL_NAME_LEN, res_str)));
        }
        else if len_check_type == StrLenCheckType::DataLenCheck
            && res_str.len() > MAX_DATA_STR_LEN
        {
            return Err(io_other_err_wrapper(&format!("read_string_from_buf - str data exceeds MAX allowed size: {}", MAX_DATA_STR_LEN)));
        }

        *curr_buf_idx += 1;
    }

    Ok(eos_present)
}