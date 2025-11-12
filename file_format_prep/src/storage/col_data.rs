use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::io::{Write};

use std::io::Error as io_err;
use crate::constants::{AllowedColTypes, BATCH_SIZE, CHUNK_SIZE_BYTES, DB_DATA_DIR};
use crate::storage::encoders::{delta_encode, vle_encode_i, vle_encode_u, vle_decode_i, vle_decode_u};
use crate::storage::metadata_structs::DbMetadata;
use crate::storage::col_header::ColHeader;

#[cfg(test)]
#[path = "../tests/test_column_structs.rs"]
mod test_column_structs;

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
    data: Vec<T>,
    n_rows: usize,
    average: f64,
    file_handle: Option<File>,
    first_time_saving: bool,
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
            data: Vec::new(),
            n_rows: 0,
            average: 0.0,
            file_handle: None,
            first_time_saving: true
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

    pub fn read_from_file(mut f: File) -> ColData<i64>
    {
        // TODO: ADD PROPER ERROR HANDLIIIING with my-defined Errors
        let mut buf = [0 as u8; CHUNK_SIZE_BYTES];
        let mut bytes_read;
        let mut buf_idx = 0;

        bytes_read = f.read(&mut buf).unwrap();

        let mut header = ColHeader::read_from_buf(&mut buf_idx, bytes_read, &buf).unwrap();

        let mut size_read = bytes_read - buf_idx;
        let mut bytes: Vec<u8> = Vec::new();
        let mut first_value = true;
        let mut result_vec: Vec<i64> = Vec::new();
        let mut min_val: i64 = 0;
        let mut n_rows: usize = 0;
        let mut average: f64 = 0.0;

        println!("col data size: {}", header.size_of_data());
        print!("decoded: ");
        loop 
        {
            println!("size_read: {size_read}");
            if buf_idx >= bytes_read
            {
                let is_break = ColData::read_new_data(
                    &mut buf_idx, 
                    &mut bytes_read, 
                    &mut size_read, 
                    &mut header, 
                    &mut buf, 
                    &mut f
                );

                if is_break {break;}
            }

            let byte = buf.get(buf_idx).unwrap();
            bytes.push(*byte);

            // If most significant bit is 0 it means that this is last byte
            // in vle encoded sequence, so we need to decode it into i64
            if byte & 0x80 == 0
            {
                let decoded_val = ColData::decode_bytes(
                                &mut first_value, 
                                &mut bytes, 
                                &mut min_val);

                result_vec.push(decoded_val);
                print!("{} ", decoded_val);

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

        println!();
        println!("average: {}", average);

        ColData {
            header: header,
            data: result_vec,
            n_rows: n_rows,
            average: average,
            file_handle: None,
            first_time_saving: false
        }
    }

    fn decode_bytes(
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

    fn read_new_data(
        buf_idx: &mut usize, 
        bytes_read: &mut usize,
        size_read: &mut usize,
        header: &mut ColHeader,
        buf: &mut [u8; CHUNK_SIZE_BYTES],    
        f: &mut File
    ) -> bool
    {
        *buf_idx = 0;
        *bytes_read = f.read(buf).unwrap();

        // Here we just add bytes_read since we are not reading header data now
        *size_read += *bytes_read;

        if *size_read > header.size_of_data() as usize
        {
            panic!("Size of data we read is greater than size of data stored in file's header");
        }

        if *bytes_read == 0 
        {
            if header.is_overflow()
            {
                // If overflow, this means that there are more files 
                // for this column. Here we will open next file, 
                // create new header and continue reading from buffer
                *f = ColData::continue_to_next_file(
                                                    header, 
                                                    buf_idx, 
                                                    bytes_read, 
                                                    buf);
                // We need to substract buf_idx since first bytes we read from
                // file are headers bytes, thus nbr of bytes of data we read
                // is bytes_read - buf_idx 
                *size_read = *bytes_read - *buf_idx;
            }
            else if *size_read != header.size_of_data() as usize
            {
                panic!("We ended reading data from file, but amount of data we read is not equal to data size stored in file header");
            }
            else 
            {
                // No more data, all was read, we want to break from main loop
                return true;
            }
        }
        return false;
    }

    fn continue_to_next_file(
        col_h: &mut ColHeader, 
        buf_idx: &mut usize,
        bytes_read: &mut usize,
        buf: &mut [u8; CHUNK_SIZE_BYTES],
    ) -> File
    {
        let mut f = File::open(col_h.get_next_file_path()).unwrap();

        *bytes_read = f.read(buf).unwrap();
        *buf_idx = 0;

        *col_h = ColHeader::read_from_buf(buf_idx, *bytes_read, buf).unwrap();

        f
    }

    pub fn create_and_save_to_file(&mut self) -> (String, File)
    {
        let encoded_vec = self.vle_encode();
        self.header.increase_data_size(encoded_vec.len() as u32).unwrap();
        let (file_name, mut f) = self.header.save_to_file(DB_DATA_DIR).unwrap();

        f.write_all(&encoded_vec).unwrap();

        (file_name, f)
    }

    pub fn append_to_file(&mut self, mut f: File)
    {
        let encoded_vec = self.vle_encode();

        f.write_all(&encoded_vec).unwrap();
    }


    /// - DB manager firstly needs to create ColHeader and then ColData so that
    /// we know to which file we need to write stuff.
    /// - DB manager will read strings from files, convert BATCH_SIZE of them 
    /// into vector of i64 and we will get this vector and will need to 
    /// serialize it and save to file
    pub fn save_to_file(&mut self, ints: &Vec<i64>)
    {
        // TODO: better error handling
        if ints.len() > BATCH_SIZE
        {
            panic!("ColData - save_to_file - vector of data has greater size than BATCH_SIZE");
        }

        let mut f: File = self.get_file_handle();

    }

    fn get_file_handle(&mut self) -> File
    {
        let f: File;
        if self.first_time_saving
        {
            self.first_time_saving = false;

            // we get file handle to created file, to which we will append data
            (_, f) = self.header.save_to_file(DB_DATA_DIR).unwrap();
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
                f = File::open(self.header.get_file_path()).unwrap();
            }
        }
        f
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
