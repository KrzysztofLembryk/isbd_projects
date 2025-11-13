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

    fn _read_new_data(
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
                *f = ColData::<T>::_continue_to_next_file(
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
    ) ->Result<File, io_err>
    {
        println!("_save_data_chunk_to_file - bytes: {}", bytes_read);
        match self.header.increase_data_size(bytes_read as u32)
        {
            Ok(_) => {
                println!("Enough space in file, we will save: {} bytes", bytes_read);
                // We will append to a file so we always know were to write
                f.seek(SeekFrom::End(0))?;
                f.write(&buf[..bytes_read])?;
                return Ok(f);
            }
            Err(free_space_size) => {
                println!("Not enough space in file, we will write only a part of a chunk, free space left to write: {} bytes", free_space_size);

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
    ) -> File
    {
        let mut buf = [0u8; CHUNK_SIZE_BYTES];
        let mut buf_idx = 0;

        // TODO: Probably we could use slicing and just jump CHUNK_SIZE_BYTES in
        // vals vector
        for c in vals
        {
            let buf_val = buf.get_mut(buf_idx).unwrap();

            *buf_val = *c;
            buf_idx += 1;

            // only when full buff we save chunk
            if buf_idx >= CHUNK_SIZE_BYTES
            {
                let bytes_read = buf_idx;

                f = self._save_data_chunk_to_file(
                    f, 
                    bytes_read, 
                    &buf).unwrap();

                buf_idx = 0;
            }
        }

        // It means that we didnt save last chunk since it wasnt of max size
        if buf_idx != 0
        {
            f = self._save_data_chunk_to_file(
                f, 
                buf_idx, 
                &buf).unwrap();
        }

        // TODO: add variable that checks this
        // We might have not updated data in header so we do it now to be sure
        self.header.modify_data_size_in_file(&mut f).unwrap();

        f
    }

    fn _continue_to_next_file(
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

    fn _get_file_handle(&mut self) -> File
    {
        let f: File;
        if self.first_time_saving
        {
            println!("first time saving");
            self.first_time_saving = false;

            // we get file handle to created file, to which we will append data
            (_, f) = self.header.save_to_file(DB_DATA_DIR).unwrap();
        }
        else 
        {
            println!("Not saving first time");
            // We're not saving for the first time, so there should be file that
            // we previously created so we can open it
            if let Some(file) = self.file_handle.take()
            {
                println!("File handle present");
                f = file;
            }
            else 
            {
                println!("file handle not present");
                f = File::open(self.header.get_file_path()).unwrap();
            }
        }
        f
    }

}

impl ColData<i64>
{
    // ########################################################################
    // ############################# PUBLIC API ###############################
    // ########################################################################
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
            if buf_idx >= bytes_read
            {
                let is_break = ColData::<i64>::_read_new_data(
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
                let decoded_val = ColData::_decode_bytes(
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
        println!();

        ColData {
            header: header,
            data: result_vec,
            n_rows: n_rows,
            average: average,
            file_handle: None, // maybe better to store f?
            first_time_saving: false
        }
    }

    /// - DB manager firstly needs to create ColHeader and then ColData so that
    /// we know to which file we need to write stuff.
    /// - DB manager will read strings from files, convert BATCH_SIZE of them 
    /// into vector of i64 and we will get this vector and will need to 
    /// serialize it and save to file
    pub fn save_to_file(&mut self, ints: &[i64])
    {
        // TODO: better error handling
        if ints.len() > BATCH_SIZE
        {
            panic!("ColData - save_to_file - vector of data has greater size than BATCH_SIZE");
        }

        println!("Saving ints: {:?}", ints);
        println!();
        let mut f: File = self._get_file_handle();
        let ints_encoded = ColData::_vle_encode(ints);

        f = self._do_the_save(&ints_encoded, f);

        self.file_handle = Some(f);
    }

    // ########################################################################
    // ############################ PRIVATE API ###############################
    // ########################################################################

    fn _vle_encode(vals: &[i64]) -> Vec<u8>
    {
        let delta_encoded_vec = delta_encode(vals);
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

//##############################################################################
//######################## PRIVATE HELPER FUNCTIONS ############################
//##############################################################################
// TODO: we shouldnt return io_err everywhere
