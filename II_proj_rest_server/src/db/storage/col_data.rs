use tokio::fs as tokio_fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt, SeekFrom, AsyncSeekExt};
use zstd;
use std::collections::VecDeque;

use crate::db::constants::{LogicalColType, BATCH_SIZE, BUF_SIZE, ZSTD_ENCODE_LEVEL};
use crate::db::storage::encoders::{delta_encode, vle_encode_i, vle_encode_u, vle_decode_i, vle_decode_u};

use crate::db::storage::col_header::ColHeader;
use crate::db::errors::DbError;

#[cfg(test)]
#[path = "../tests/storage/test_col_data.rs"]
mod test_col_data;


#[derive(PartialEq)]
enum ReadStage 
{
    SizeStage,
    DataStage
}

pub trait ColType 
{
    fn col_type() -> LogicalColType;
}

impl ColType for i64
{
    fn col_type() -> LogicalColType {
        LogicalColType::INT64
    }
}

impl ColType for String
{
    fn col_type() -> LogicalColType {
        LogicalColType::VARCHAR 
    }
}

#[derive(Debug)]
pub struct ColData<T: ColType>
{
    header: ColHeader,
    data: Vec<T>,
    n_rows: usize,
    file_handle: Option<tokio::fs::File>,
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
            // result: ResType::get_res_type::<T>(),
            file_handle: None,
            first_time_saving: true,
        })
    }

    pub fn n_rows(&self) -> usize
    {
        self.n_rows
    }

    pub fn col_file_id(&self) -> u16
    {
        self.header.col_id()
    }

    pub fn col_name(&self) -> &str
    {
        self.header.col_name()
    }

    /// Function **consumes SELF**
    pub fn data(self) -> Vec<T>
    {
        self.data
    }

    async fn _read_new_data(
        buf_idx: &mut usize, 
        bytes_read: &mut usize,
        size_read: &mut usize,
        header: &mut ColHeader,
        remaining_files: &mut VecDeque<&str>,
        buf: &mut [u8],    
        f: &mut tokio_fs::File,
        dir_path: &str
    ) -> Result<bool, DbError>
    {
        *buf_idx = 0;
        *bytes_read = f.read(buf).await?;

        // Here we just add bytes_read since we are not reading header data now
        *size_read += *bytes_read;

        if *size_read > header.size_of_data() as usize
        {
            return Err(DbError::SizeMismatch {
                msg: format!("ColData::_read_new_data - Size of data read from file of column: {} exceeds size stored in file header", header.col_name()),
                size_1: *size_read,
                size_2: header.size_of_data() as usize
            });
        }

        if *bytes_read == 0 
        {
            if let Some(file_path) = remaining_files.pop_front()
            {
                *f = tokio_fs::File::open(file_path).await?;
                *bytes_read = f.read(buf).await?;
                *buf_idx = 0;
                *header = ColHeader::create_header_from_buf(
                    T::col_type(),
                    buf_idx, 
                    *bytes_read, 
                    buf,
                    dir_path
                )?;
                // We need to substract buf_idx since first bytes we read from
                // file are headers bytes, thus nbr of data bytes we read
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
        // Still some data left to read
        return Ok(false);
    }

    /// This function TAKES OWNERSHIP of **f**: File. <br>
    /// It returns either the same f or a file hook to newly created file
    /// - Function appends bytes_read bytes to a given file
    /// - If given file has to little space, it creates new one while also 
    /// updating metadata and creating new ColHeader object
    async fn _save_data_chunk_to_file(
        &mut self,
        mut f: tokio_fs::File,            
        bytes_read: usize,
        buf: &[u8],
    ) ->Result<tokio_fs::File, DbError>
    {
        match self.header.increase_data_size(bytes_read as u32)
        {
            Ok(_) => {
                // We will append to a file so we always know were to write
                f.seek(SeekFrom::End(0)).await?;
                f.write(&buf[..bytes_read]).await?;
                return Ok(f);
            }
            Err(available_space) => {

                // we save updated col_header to a file
                self.header.modify_data_size_in_file(&mut f).await?;

                // Not enough free space in file, thus we will save as much as 
                // we can and will create a new file 
                f.seek(SeekFrom::End(0)).await?;
                f.write(&buf[..available_space]).await?;

                // We no longer need old col_header, we will write to a new file
                self.header = self.header.create_next()?;
                let (_, mut new_f) = self.header.save_to_file().await?;

                // We do save
                let size_of_rest_of_data: usize = bytes_read - available_space;
                
                self.header.increase_data_size(size_of_rest_of_data as u32).unwrap();
                new_f.seek(SeekFrom::End(0)).await?;
                new_f.write(&buf[available_space..bytes_read]).await?;

                return Ok(new_f);
            }
        }
    }

    async fn _do_the_save(
        &mut self,
        vals: &[u8],
        mut f: tokio_fs::File,
    ) -> Result<tokio_fs::File, DbError>
    {
        let mut curr_pos: usize = 0;
        let vals_len = vals.len();
        // TODO: Probably we could use slicing and just jump CHUNK_SIZE_BYTES in
        // vals vector
        while curr_pos < vals_len
        {
            let end_pos = std::cmp::min(curr_pos + BUF_SIZE, vals_len);
            let chunk = &vals[curr_pos..end_pos];
            let chunk_len = chunk.len();

            f = self._save_data_chunk_to_file(
                f, 
                chunk_len, 
                &chunk).await?;

            curr_pos += chunk_len;
        }
        self.header.modify_data_size_in_file(&mut f).await?;

        Ok(f)
    }

    async fn _get_file_handle(&mut self) -> Result<tokio_fs::File, DbError>
    {
        let f: tokio_fs::File;
        if self.first_time_saving
        {
            self.first_time_saving = false;

            // we get file handle to created file, to which we will append data
            (_, f) = self.header.save_to_file().await?;
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
                f = tokio_fs::File::open(self.header.get_file_path()).await?;
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
    pub async fn read_from_file(
        mut remaining_files: VecDeque<&str>,
        dir_path: &str
    ) -> Result<ColData<i64>, DbError>
    {
        let mut buf = vec![0u8; BUF_SIZE];
        let mut bytes_read;
        let mut buf_idx = 0;

        let file_path = match remaining_files.pop_front()
        {
            Some(path) => path,
            None => {
                // put_table endpoint takes care of checking if provided 
                // table schema has at least one column thus file paths
                return Err(DbError::InternalDbError(format!("ColData<i64>::read_from_file: provided queue of file_paths is empty")));
            }
        };

        let mut f = tokio_fs::File::open(file_path).await?;

        bytes_read = f.read(&mut buf).await?;

        let mut header = ColHeader::create_header_from_buf(
            i64::col_type(),
            &mut buf_idx, 
            bytes_read, 
            &buf,
            dir_path
        )?;

        // Below var checks how many bytes of DATA we read, not metadata
        // thus we substract buf_idx since it now stores metadata count
        let mut data_bytes_read = bytes_read - buf_idx;
        let mut bytes: Vec<u8> = Vec::new();
        let mut first_value = true;
        let mut result_vec: Vec<i64> = Vec::new();
        let mut min_val: i64 = 0;
        let mut n_rows: usize = 0;
        let mut n_rows_in_batch: usize = 0;
        let mut curr_batch_size: u16 = 0;
        let mut read_stage = ReadStage::SizeStage;

        loop 
        {
            if buf_idx >= bytes_read
            {
                // Reads new data, if needed opens next file in queue
                let is_break = ColData::<i64>::_read_new_data(
                    &mut buf_idx, 
                    &mut bytes_read, 
                    &mut data_bytes_read, 
                    &mut header, 
                    &mut remaining_files,
                    &mut buf, 
                    &mut f,
                    dir_path
                ).await?;

                if is_break {break;}
            }

            let byte = buf
                .get(buf_idx)
                .ok_or_else(|| DbError::InternalDbError(
                    format!("ColData<i64>::read_from_file - buf_idx {} out of bounds for buffer (len: {})", buf_idx, buf.len())
                ))?;
            bytes.push(*byte);

            match read_stage
            {
                ReadStage::SizeStage => {
                    if bytes.len() == 2
                    {
                        curr_batch_size = 
                            u16::from_be_bytes([bytes[0], bytes[1]]);
                        
                        read_stage = ReadStage::DataStage;
                        bytes.clear();
                    }
                },
                ReadStage::DataStage => {
                    // If most significant bit is 0 it means that this is last byte
                    // in vle encoded sequence, so we need to decode it into i64
                    if byte & 0x80 == 0
                    {
                        // delta encoding value is always first in sequence
                        let not_delta_encoding_base_value = !first_value;
                        let decoded_val = ColData::_decode_bytes(
                                        &mut first_value, 
                                        &mut bytes, 
                                        &mut min_val);

                        bytes.clear();
                        // First value in a sequence is needed only for 
                        // decoding, therefore we do not push it since it would 
                        // destroy order of our data
                        if not_delta_encoding_base_value
                        {
                            result_vec.push(decoded_val);
                            n_rows_in_batch += 1;
                            n_rows += 1;

                            if n_rows_in_batch == curr_batch_size as usize
                            {
                                // first two bytes in every batch is batch size
                                // so after reading all data from current batch
                                // we change read stage to size stage
                                read_stage = ReadStage::SizeStage;
                                n_rows_in_batch = 0;
                                first_value = true;
                            }
                        }
                    }
                }
            }

            buf_idx += 1;
        }

        Ok(ColData {
            header: header,
            data: result_vec,
            n_rows: n_rows,
            file_handle: None, 
            first_time_saving: false,
        })
    }

    /// - DB manager firstly needs to create ColHeader and then ColData so that
    /// we know to which file we need to write stuff.
    /// - DB manager will read strings from files, convert BATCH_SIZE of them 
    /// into vector of i64 and we will get this vector and will need to 
    /// serialize it and save to file
    pub async fn save_to_file(&mut self, ints: &[i64]) -> Result<(), DbError>
    {
        if ints.len() > BATCH_SIZE
        {
            return Err(DbError::InternalDbError(
                format!("ColData<INT64> - save_to_file - provided vector of ints has greater size: '{}' than BATCH_SIZE: {}", ints.len(),BATCH_SIZE
            )));
        }

        let mut f = self._get_file_handle().await?;
        let ints_encoded = ColData::_vle_encode(ints)?;
        let rows_in_batch: u16 = ints.len() as u16;

        let mut data_to_save = Vec::with_capacity(std::mem::size_of::<u16>() + ints_encoded.len());

        data_to_save.extend_from_slice(&rows_in_batch.to_be_bytes());
        data_to_save.extend_from_slice(&ints_encoded);
        
        f = self._do_the_save(&data_to_save, f).await?;

        self.file_handle = Some(f);

        Ok(())
    }

    // ########################################################################
    // ############################ PRIVATE API ###############################
    // ########################################################################

    fn _vle_encode(vals: &[i64]) -> Result<Vec<u8>, DbError>
    {
        // delta_encoded_vec has size BATCH_SIZE + 1 if vals.len() == BATCH_SIZE
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

            
            *min_val = decoded_val;
            *first_value = false;
        }
        else 
        {
            // We did delta encoding so we need to add min_val to our decoded 
            // value to get final result
            decoded_val = vle_decode_u(&bytes) as i64;
            decoded_val += *min_val;

        }

        decoded_val
    }
}

impl ColData<String>
{
    pub async fn read_from_file(
        mut remaining_files: VecDeque<&str>,
        dir_path: &str,
    ) -> Result<ColData<String>, DbError>
    {
        // for reading all given column files, we do one buffer allocation
        let mut buf = vec![0u8; BUF_SIZE];
        let mut bytes_read;
        let mut buf_idx = 0;

        let file_path = pop_first_path(&mut remaining_files)?;
        let mut f = tokio_fs::File::open(file_path).await?;

        bytes_read = f.read(&mut buf).await?;

        let mut header = ColHeader::create_header_from_buf(
                                            String::col_type(),
                                            &mut buf_idx, 
                                            bytes_read, 
                                            &buf,
                                            dir_path
                                        )?;
        // Below var checks how many bytes of DATA we read, not metadata
        // thus we substract buf_idx since it now stores metadata count
        let mut data_bytes_read = bytes_read - buf_idx;
        let mut bytes: Vec<u8> = Vec::new();

        let mut result_vec: Vec<String> = Vec::new();
        let mut n_rows: usize = 0;

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
                    &mut data_bytes_read, 
                    &mut header, 
                    &mut remaining_files,
                    &mut buf, 
                    &mut f,
                    dir_path
                ).await?;

                if is_break {
                    break;
                }
            }

            let byte = buf
                .get(buf_idx)
                .ok_or_else(|| DbError::Other(
                    format!("ColData<String>::read_from_file - buf_idx {} out of bounds for buffer (len: {})", buf_idx, buf.len())
                ))?;
            bytes.push(*byte);
            n_bytes += 1;

            if curr_stage == ReadStage::SizeStage 
            && n_bytes == nbr_of_size_bytes
            {
                ColData::<String>::handle_size_stage(
                    &mut bytes, 
                    &mut n_bytes, 
                    &mut curr_stage, 
                    &mut zstd_frame_size
                );
            }
            else if n_bytes == zstd_frame_size
            {
                // We can decode strings only when we have read exactly 
                // zstd_frame_size bytes
                ColData::<String>::handle_data_stage(
                    &mut result_vec, 
                    &mut bytes, 
                    &mut n_bytes, 
                    &mut n_rows, 
                    &mut curr_stage
                )?;
            }

            buf_idx += 1;
        }

        Ok(ColData {
            header: header,
            data: result_vec,
            n_rows: n_rows,
            file_handle: None, 
            first_time_saving: false,
        })
    }

    pub async fn save_to_file(&mut self, strings: &[String]) -> Result<(), DbError>
    {
        if strings.len() > BATCH_SIZE
        {
            return Err(DbError::SizeExceeded {
                msg: "ColData<String>::save_to_file - vector of data exceeds BATCH_SIZE".to_string(),
                max: BATCH_SIZE
            });
        }

        let mut f = self._get_file_handle().await?;
        let strs_encoded = ColData::_zstd_encode(strings)?;
        f = self._do_the_save(&strs_encoded, f).await?;

        self.file_handle = Some(f);

        Ok(())
    }

    fn handle_data_stage(
        result_vec: &mut Vec<String>,
        bytes: &mut Vec<u8>,
        n_bytes: &mut u32,
        n_rows: &mut usize,
        curr_stage:  &mut ReadStage,
    ) -> Result<(), DbError>
    {
        let decoded_strings = ColData::<String>::_zstd_decode(bytes)?;
        // Each decoded string is a value for separate row
        *n_rows += decoded_strings.len();
        result_vec.extend(decoded_strings);

        // After reading one full zstd frame, we again need to read
        // frame size, so we switch stage
        *n_bytes = 0;
        *curr_stage = ReadStage::SizeStage;
        bytes.clear();

        return Ok(());
    }

    fn handle_size_stage(
        bytes: &mut Vec<u8>,
        n_bytes: &mut u32,
        curr_stage:  &mut ReadStage,
        zstd_frame_size: &mut u32,
    )
    {
        if bytes.len() != 4
        {
            panic!("sth went wrong, in bytes we don't have 4 bytes in buf, but {}", bytes.len());
        }

        // In bytes we should have only 4 bytes 
        *zstd_frame_size = u32::from_be_bytes(bytes[..4].try_into().unwrap());
        *n_bytes = 0;
        *curr_stage = ReadStage::DataStage;

        bytes.clear();
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
        
        // We save and read BUF_SIZE bytes, thus we may not save whole 
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


fn pop_first_path<'a>(remaining_files: &mut VecDeque<&'a str>) -> Result<&'a str, DbError>
{
    return match remaining_files.pop_front()
    {
        Some(path) => Ok(path),
        None => {
            Err(DbError::Other(format!("ColData<String>::read_from_file: provided queue of file_paths is empty")))
        }
    };
}