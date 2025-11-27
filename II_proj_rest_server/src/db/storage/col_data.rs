use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::io::{Write};
use zstd;

use crate::db::constants::{LogicalColType, BATCH_SIZE, CHUNK_SIZE_BYTES, DB_DATA_DIR, ZSTD_ENCODE_LEVEL};
use crate::db::storage::encoders::{delta_encode, vle_encode_i, vle_encode_u, vle_decode_i, vle_decode_u};

// TODO: metadata should be updated when we save to file
// use crate::storage::metadata_structs::DbMetadata;

use crate::db::storage::col_header::ColHeader;
use crate::db::errors::DbError;

#[cfg(test)]
#[path = "../tests/test_column_structs.rs"]
mod test_column_structs;

enum ResType
{
    StrColRes(usize),
    IntColRes(f64)
}

impl ResType
{
    pub fn get_res_type<T: ColType>() -> ResType
    {
        match T::col_type()
        {
            LogicalColType::IntType => ResType::IntColRes(0.0),
            LogicalColType::StrType => ResType::StrColRes(0)
        }
    }
}

pub trait ColType 
{
    fn col_type() -> LogicalColType;
}

impl ColType for i64
{
    fn col_type() -> LogicalColType {
        LogicalColType::IntType
    }
}

impl ColType for String
{
    fn col_type() -> LogicalColType {
        LogicalColType::StrType 
    }
}

pub struct ColData<T: ColType>
{
    header: ColHeader,
    data: Vec<T>,
    n_rows: usize,
    result: ResType,
    file_handle: Option<File>,
    first_time_saving: bool,
}

impl<T: ColType> ColData<T>
{
    pub fn new(header: ColHeader) -> Result<ColData<T>, DbError>
    {
        if header.col_type() != T::col_type()
        {
            return Err(DbError::ColumnTypeMismatch(
                format!("ColData::new: Column type mismatch btwn header and data type")));
        }

        Ok(ColData {
            header: header,
            data: Vec::new(),
            n_rows: 0,
            result: ResType::get_res_type::<T>(),
            file_handle: None,
            first_time_saving: true
        })
    }

    pub fn n_rows(&self) -> usize
    {
        self.n_rows
    }

    fn _read_new_data(
        buf_idx: &mut usize, 
        bytes_read: &mut usize,
        size_read: &mut usize,
        header: &mut ColHeader,
        buf: &mut [u8; CHUNK_SIZE_BYTES],    
        f: &mut File
    ) -> Result<bool, DbError>
    {
        *buf_idx = 0;
        *bytes_read = f.read(buf)?;

        // Here we just add bytes_read since we are not reading header data now
        *size_read += *bytes_read;

        if *size_read > header.size_of_data() as usize
        {
            return Err(DbError::SizeMismatch {
                msg: "ColData::_read_new_data - Size of data read exceeds size stored in file header".to_string(),
                size_1: *size_read,
                size_2: header.size_of_data() as usize
            });
        }

        if *bytes_read == 0 
        {
            if header.is_overflow()
            {
                // If overflow, this means that there are more files 
                // for this column. Here we will open next file, 
                // create new header and continue reading from buffer
                *f = ColData::<T>::_continue_to_next_file(
                                                    header, 
                                                    buf_idx, 
                                                    bytes_read, 
                                                    buf)?;
                // We need to substract buf_idx since first bytes we read from
                // file are headers bytes, thus nbr of bytes of data we read
                // is bytes_read - buf_idx 
                *size_read = *bytes_read - *buf_idx;
            }
            else if *size_read != header.size_of_data() as usize
            {
                return Err(DbError::Other("ColData::_read_new_data: We ended reading data from file, but amount of data we read is not equal to data size stored in file header".to_string()));
            }
            else 
            {
                // No more data, all was read, we want to break from main loop
                return Ok(true);
            }
        }
        return Ok(false);
    }

    /// This function TAKES OWNERSHIP of **f**: File. <br>
    /// It returns either the same f or a file hook to newly created file
    /// - Function appends bytes_read bytes to a given file
    /// - If given file has to little space, it creates new one while also 
    /// updating metadata and creating new ColHeader object
    fn _save_data_chunk_to_file(
        &mut self,
        mut f: File,            
        bytes_read: usize,
        buf: &[u8],
    ) ->Result<File, DbError>
    {
        match self.header.increase_data_size(bytes_read as u32)
        {
            Ok(_) => {
                // We will append to a file so we always know were to write
                f.seek(SeekFrom::End(0))?;
                f.write(&buf[..bytes_read])?;
                return Ok(f);
            }
            Err(free_space_size) => {

                // we save updated col_header to a file
                self.header.modify_data_size_in_file(&mut f)?;

                // Not enough free space in file, thus we will save as much as 
                // we can and will create a new file 
                f.seek(SeekFrom::End(0))?;
                f.write(&buf[..free_space_size])?;

                // We no longer need old col_header, we will write to a new file
                self.header = self.header.create_next()?;
                let (_, new_f) = self.header.save_to_file(DB_DATA_DIR)?;

                // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                // !!!!! TODO: REMEMBER TO UPDATE METADATA !!!!!!
                // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

                // And now we recursively invoke this function, since now we 
                // will go into OK branch
                return self._save_data_chunk_to_file(
                    new_f, 
                    bytes_read - free_space_size, 
                    &buf[free_space_size..bytes_read]);
            }
        }
    }

    fn _do_the_save(
        &mut self,
        vals: &[u8],
        mut f: File,
    ) -> Result<File, DbError>
    {
        let mut buf = [0u8; CHUNK_SIZE_BYTES];
        let mut buf_idx = 0;

        // TODO: Probably we could use slicing and just jump CHUNK_SIZE_BYTES in
        // vals vector
        for c in vals
        {
            let buf_val = buf
                .get_mut(buf_idx)
                .ok_or_else(|| DbError::Other(
                    format!("ColData::_do_the_save - buf_idx {} out of bounds for buffer (len: {})", buf_idx, CHUNK_SIZE_BYTES)))?;

            *buf_val = *c;
            buf_idx += 1;

            // only when full buff we save chunk
            if buf_idx >= CHUNK_SIZE_BYTES
            {
                let bytes_read = buf_idx;

                f = self._save_data_chunk_to_file(
                    f, 
                    bytes_read, 
                    &buf)?;

                buf_idx = 0;
            }
        }

        // It means that we didnt save last chunk since it wasnt of max size
        if buf_idx != 0
        {
            f = self._save_data_chunk_to_file(
                f, 
                buf_idx, 
                &buf)?;
        }

        // TODO: add variable that checks this
        // We might have not updated data in header so we do it now to be sure
        self.header.modify_data_size_in_file(&mut f)?;

        Ok(f)
    }

    fn _continue_to_next_file(
        col_h: &mut ColHeader, 
        buf_idx: &mut usize,
        bytes_read: &mut usize,
        buf: &mut [u8; CHUNK_SIZE_BYTES],
    ) -> Result<File, DbError>
    {
        let mut f = File::open(col_h.get_next_file_path()?)?;

        *bytes_read = f.read(buf)?;
        *buf_idx = 0;

        *col_h = ColHeader::read_from_buf(buf_idx, *bytes_read, buf)?;

        Ok(f)
    }

    fn _get_file_handle(&mut self) -> Result<File, DbError>
    {
        let f: File;
        if self.first_time_saving
        {
            self.first_time_saving = false;

            // we get file handle to created file, to which we will append data
            (_, f) = self.header.save_to_file(DB_DATA_DIR)?;
        }
        else 
        {
            // We're not saving for the first time, so there should be file that
            // we previously created so we can open it
            if let Some(file) = self.file_handle.take()
            {
                f = file;
            }
            else 
            {
                f = File::open(self.header.get_file_path())?;
            }
        }
        Ok(f)
    }

}

impl ColData<i64>
{
    // ########################################################################
    // ############################# PUBLIC API ###############################
    // ########################################################################
    pub fn read_from_file(mut f: File) -> Result<ColData<i64>, DbError>
    {
        // TODO: ADD PROPER ERROR HANDLIIIING with my-defined Errors
        let mut buf = [0 as u8; CHUNK_SIZE_BYTES];
        let mut bytes_read;
        let mut buf_idx = 0;

        bytes_read = f.read(&mut buf)?;

        let mut header = ColHeader::read_from_buf(
            &mut buf_idx, 
            bytes_read, 
            &buf
        )?;

        // Below var checks how many bytes of DATA we read, not metadata
        // thus we substract buf_idx since it now stores metadata count
        let mut size_data_bytes_read = bytes_read - buf_idx;
        let mut bytes: Vec<u8> = Vec::new();
        let mut first_value = true;
        let mut result_vec: Vec<i64> = Vec::new();
        let mut min_val: i64 = 0;
        let mut n_rows: usize = 0;
        let mut average: f64 = 0.0;

        loop 
        {
            if buf_idx >= bytes_read
            {
                let is_break = ColData::<i64>::_read_new_data(
                    &mut buf_idx, 
                    &mut bytes_read, 
                    &mut size_data_bytes_read, 
                    &mut header, 
                    &mut buf, 
                    &mut f
                )?;

                if is_break {break;}
            }

            let byte = buf
                .get(buf_idx)
                .ok_or_else(|| DbError::Other(
                    format!("ColData<i64>::read_from_file - buf_idx {} out of bounds for buffer (len: {})", buf_idx, buf.len())
                ))?;
            bytes.push(*byte);

            // If most significant bit is 0 it means that this is last byte
            // in vle encoded sequence, so we need to decode it into i64
            if byte & 0x80 == 0
            {
                let decoded_val = ColData::_decode_bytes(
                                &mut first_value, 
                                &mut bytes, 
                                &mut min_val);

                result_vec.push(decoded_val);

                average = ((average * n_rows as f64) + decoded_val as f64) 
                        / ((n_rows + 1) as f64);
                n_rows += 1;
                
                if n_rows % BATCH_SIZE == 0
                {
                    // When we start new batch again first value might be negative
                    first_value = true;
                }
            }
            buf_idx += 1;
        }

        Ok(ColData {
            header: header,
            data: result_vec,
            n_rows: n_rows,
            result: ResType::IntColRes(average),
            file_handle: None, // maybe better to store f?
            first_time_saving: false
        })
    }

    /// - DB manager firstly needs to create ColHeader and then ColData so that
    /// we know to which file we need to write stuff.
    /// - DB manager will read strings from files, convert BATCH_SIZE of them 
    /// into vector of i64 and we will get this vector and will need to 
    /// serialize it and save to file
    pub fn save_to_file(&mut self, ints: &[i64]) -> Result<(), DbError>
    {
        // TODO: better error handling
        if ints.len() > BATCH_SIZE
        {
            panic!("ColData - save_to_file - vector of data has greater size than BATCH_SIZE");
        }

        let mut f: File = self._get_file_handle()?;
        let ints_encoded = ColData::_vle_encode(ints)?;

        f = self._do_the_save(&ints_encoded, f)?;

        self.file_handle = Some(f);

        Ok(())
    }


    pub fn result(&self) -> f64
    {
        match self.result
        {
            ResType::IntColRes(val) => val,
            _ => panic!("ColData<i64> has not IntColRes as result type")
        }
    }
    // ########################################################################
    // ############################ PRIVATE API ###############################
    // ########################################################################

    fn _vle_encode(vals: &[i64]) -> Result<Vec<u8>, DbError>
    {
        let delta_encoded_vec = delta_encode(vals)?;
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

        Ok(vle_encoded_vec)
    }

    fn _decode_bytes(
        first_value: &mut bool, 
        bytes: &mut Vec<u8>,
        min_val: &mut i64,
    ) -> i64
    {
        let mut decoded_val: i64;

        if *first_value
        {
            // First value might be negative, and is minimal value in Batch
            decoded_val = vle_decode_i(bytes);

            bytes.clear();
            
            *min_val = decoded_val;
            *first_value = false;
        }
        else 
        {
            // We did delta encoding so we need to add min_val to our decoded 
            // value to get final result
            decoded_val = vle_decode_u(&bytes) as i64;
            decoded_val += *min_val;

            bytes.clear();
        }

        decoded_val
    }
}

impl ColData<String>
{
    pub fn read_from_file(mut f: File) -> Result<ColData<String>, DbError>
    {
        // TODO: ADD PROPER ERROR HANDLIIIING with my-defined Errors
        let mut buf = [0 as u8; CHUNK_SIZE_BYTES];
        let mut bytes_read;
        let mut buf_idx = 0;

        bytes_read = f.read(&mut buf)?;

        let mut header = ColHeader::read_from_buf(
                                            &mut buf_idx, 
                                            bytes_read, 
                                            &buf
                                        )?;

        // Below var checks how many bytes of DATA we read, not metadata
        // thus we substract buf_idx since it now stores metadata count
        let mut size_data_bytes_read = bytes_read - buf_idx;
        let mut bytes: Vec<u8> = Vec::new();

        let result_vec: Vec<String> = Vec::new();
        let mut ascii_count: usize = 0;
        let mut n_rows: usize = 0;

        #[derive(PartialEq)]
        enum ReadStage 
        {
            SizeStage,
            DataStage
        }

        let mut n_bytes: u32 = 0;
        let mut curr_stage = ReadStage::SizeStage;
        let mut zstd_frame_size: u32 = 0;
        let nbr_of_size_bytes: u32 = std::mem::size_of::<u32>() as u32;

        loop 
        {
            if buf_idx >= bytes_read
            {
                let is_break = ColData::<String>::_read_new_data(
                    &mut buf_idx, 
                    &mut bytes_read, 
                    &mut size_data_bytes_read, 
                    &mut header, 
                    &mut buf, 
                    &mut f
                )?;

                if is_break {break;}
            }

            let byte = buf
                .get(buf_idx)
                .ok_or_else(|| DbError::Other(
                    format!("ColData<String>::read_from_file - buf_idx {} out of bounds for buffer (len: {})", buf_idx, buf.len())
                ))?;
            bytes.push(*byte);
            n_bytes += 1;

            if curr_stage == ReadStage::SizeStage
            {
                if n_bytes == nbr_of_size_bytes
                {
                    if bytes.len() != 4
                    {
                        panic!("sth went wrong, in bytes we don't have 4 bytes in buf, but {}", bytes.len());
                    }

                    // In bytes we should have only 4 bytes 
                    zstd_frame_size = u32::from_be_bytes(bytes[..4].try_into().unwrap());
                    n_bytes = 0;
                    curr_stage = ReadStage::DataStage;

                    bytes.clear();
                }
            }
            else 
            {
                // We can decode strings only when we have read exactly 
                // zstd_frame_size bytes
                if n_bytes == zstd_frame_size
                {
                    let decoded_strings = ColData::<String>::_zstd_decode(&bytes)?;

                    // Each decoded string is a value for separate row
                    n_rows += decoded_strings.len();
                    ascii_count += decoded_strings
                                    .iter()
                                    .fold(0, |mut acc, s| {
                                        acc += s.len(); 
                                        acc
                                    });

                    // After reading one full zstd frame, we again need to read
                    // frame size, so we switch stage
                    n_bytes = 0;
                    curr_stage = ReadStage::SizeStage;
                    bytes.clear();
                }
            }

            buf_idx += 1;
        }

        Ok(ColData {
            header: header,
            data: result_vec,
            n_rows: n_rows,
            result: ResType::StrColRes(ascii_count),
            file_handle: None, // maybe better to store f?
            first_time_saving: false
        })
    }

    pub fn save_to_file(&mut self, strings: &[String]) -> Result<(), DbError>
    {
        if strings.len() > BATCH_SIZE
        {
            return Err(DbError::SizeExceeded {
                msg: "ColData<String>::save_to_file - vector of data exceeds BATCH_SIZE".to_string(),
                max: BATCH_SIZE
            });
        }

        let mut f: File = self._get_file_handle()?;
        let strs_encoded = ColData::_zstd_encode(strings)?;

        f = self._do_the_save(&strs_encoded, f)?;

        self.file_handle = Some(f);

        Ok(())
    }

    pub fn result(&self) -> usize
    {
        match self.result
        {
            ResType::StrColRes(val) => val,
            _ => panic!("ColData<String> has not StrColRes as result type")
        }
    }

    fn _zstd_decode(bytes: &[u8]) -> Result<Vec<String>, DbError>
    {
        let decoded = zstd::decode_all(bytes)
            .map_err(|e| DbError::DecompressionError(
                format!("ColData<String>::_zstd_decode - zstd decompression failed: {}", e)
            ))?;
        
        let combined = String::from_utf8(decoded)
            .map_err(|e| DbError::DecompressionError(
                format!("ColData<String>::_zstd_decode - invalid UTF-8 data: {}", e)
            ))?;
        let str_vec: Vec<String> = combined
                    .split('\0')
                    .map(|s| s.to_string())
                    .collect();

        // When we encode we make sure that characters are ASCII, but someone
        // might give us malicious file/bytes and there might be non-ASCII 
        // characetrs there, so after decoding we need to check that
        if str_vec.iter().any(|s| !s.is_ascii())
        {
            return Err(DbError::DecompressionError(
                "ColData<String>::_zstd_decode - decoded strings contain non-ASCII characters".to_string()
            ));
        }

        Ok(str_vec)
    }

    fn _zstd_encode(strings: &[String]) -> Result<Vec<u8>, DbError>
    {
        if strings.iter().any(|s| !s.is_ascii())
        {
            return Err(DbError::CompressionError(
                "ColData<String>::_zstd_encode - data contains non-ASCII characters".to_string()
            ));
        }

        // We add separator so that we can easily read strings after 
        // decompression
        let concat_strs = strings.join("\0");
        let compressed = zstd::encode_all(
            concat_strs.as_bytes(), ZSTD_ENCODE_LEVEL)
            .map_err(|e| DbError::CompressionError(
                format!("ColData<String>::_zstd_encode - zstd compression failed: {}", e)
            ))?;
        
        // We save and read CHUNK_SIZE bytes, thus we may not save whole 
        // compressed data in one go and in one file, or we may not read whole
        // zstd encoded frame, thus we need to store compressed data size + data
        // cause otherwise we will not be able to decompress correctly our data
        let size: u32 = compressed.len() as u32;

        // So that we do only one malloc
        let mut result = Vec::with_capacity(std::mem::size_of::<u32>() + compressed.len());

        result.extend_from_slice(&size.to_be_bytes());
        result.extend_from_slice(&compressed);

        Ok(result)
    }
}
