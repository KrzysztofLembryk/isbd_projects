use crate::storage::column_structs::{ColHeader, ColData};
use crate::storage::metadata_structs::DbMetadata;
use crate::constants::{METADATA_FILE_PATH, DB_DATA_DIR, AllowedColTypes, CHUNK_SIZE_BYTES};
use crate::csv_reader;

use std::collections::HashMap;
use std::fs::File;
use std::io::{Error as io_err, Write};
use std::io::ErrorKind as err_kind;
use std::io::{Seek, SeekFrom};

pub struct DbManager
{
    db_meta: Option<DbMetadata>,
    col_data_map: HashMap<String, ColData>,
    metadata_dir_path: String,
    col_dir_path: String
}

impl DbManager
{
    pub fn new() -> DbManager
    {
        DbManager{
            db_meta: None,
            col_data_map: HashMap::new(),
            metadata_dir_path: String::from(METADATA_FILE_PATH),
            col_dir_path: String::from(DB_DATA_DIR),
        }
    }

    /// Currently naive implementation just to create some files for our db
    pub fn init_from_csv(&mut self, csv_path: &str) -> Result<(), String>
    {
        let (types, names, col_data) = csv_reader::read_csv(csv_path, b'\t');

        let metadata = match DbMetadata::new(types, names)
        {
            Ok(m) => m,
            Err(e) =>  return Err(format!("init_from_csv - metadata::new - {}", e))
        };

        match metadata.save_to_file(METADATA_FILE_PATH)
        {
            Ok(_) => (),
            Err(e) => return Err(format!("init_from_csv - metdata::save_to_file - {}", e))
        }

        let col_names = metadata.col_names();
        let col_types = metadata.col_types();

        let mut buf = [0; CHUNK_SIZE_BYTES];
        let buf_len = buf.len();
        let mut buf_idx = 0;
        let mut vals: Vec<u8>; 

        // What this loop does: 
        // - In col_data we store vectors of strings, each vector stores data
        //   for separate column
        // - We get col_type and col_name so that we can create new col_header
        //   and save it to file; in the same file we will also save col_data
        //   by appending to this file
        // - Then we iterate over col_data_vec and check if value should be 
        //   string or i64, we make array of bytes from it and pass it to 
        //   populate_buf function that iterates over this array of bytes
        // - If populate_buf buf fills buf it saves data chunk to a file
        for (idx, col_data_vec) in col_data.iter().enumerate()
        {
            
            let col_type = AllowedColTypes::from_u8(*col_types.get(idx).unwrap())?;
            let col_name = col_names.get(idx).unwrap().clone();
            let mut col_h = ColHeader::new_empty(col_type, col_name).unwrap();

            let (_, mut f) = col_h.save_to_file(DB_DATA_DIR).unwrap();

            for val in col_data_vec
            {
                if col_type == AllowedColTypes::StrType
                {
                    vals = val.as_bytes().try_into().unwrap();
                    // Read strings are not null terminated, we need to add this
                    // by ourselves
                    vals.push(b'\0');
                }
                else
                {
                    let int_val: i64 = val.parse().unwrap();
                    vals = int_val.to_be_bytes().try_into().unwrap();
                }

                f = self.populate_buf(
                    &mut buf_idx, 
                    buf_len, 
                    &mut buf, 
                    &vals, 
                    f, 
                    &mut col_h);
            }

            // We have written all data apart from last chunk (buf_idx was not 
            // zeroed in populate buf) to the file; this chunk is already whole 
            // in buf 
            if idx == col_data.len() - 1 && buf_idx != 0
            {
                let _ = self.save_data_chunk_to_file(
                    f, 
                    &mut col_h, 
                    buf_idx, 
                    &mut buf
                );
            }
        }

        match metadata.save_to_file(METADATA_FILE_PATH)
        {
            Ok(()) => (),
            Err(e) => return Err(format!("second metdata save to file, {}", e))
        }
        self.db_meta = Some(metadata);

        Ok(())
    }

    fn populate_buf(
        &mut self,
        buf_idx: &mut usize, 
        buf_len: usize, 
        buf: &mut [u8; 64],
        vals: &[u8],
        mut f: File,
        col_h: &mut ColHeader
    ) -> File
    {
        for c in vals
        {
            let buf_val = buf.get_mut(*buf_idx).unwrap();

            *buf_val = *c;
            *buf_idx += 1;

            // only when full buff we save chunk
            if *buf_idx >= buf_len
            {
                f = self.save_data_chunk_to_file(
                    f, 
                    col_h, 
                    *buf_idx, 
                    buf).unwrap();

                *buf_idx = 0;
            }
        }

        // it may happen that loop ends and we didnt save, cause buff was not full, but thats intended
        f
    }

    pub fn start_db(&mut self) -> Result<(), io_err>
    {
        // To start db, db metadata file must be present
        self.db_meta = Some(
            match DbMetadata::read_from_file(&self.metadata_dir_path)
            {
                Ok(meta) => meta,
                Err(e) => {
                    // if there is no metadata file, we create a new one
                    if e.kind() == err_kind::NotFound 
                    {
                        let db = DbMetadata::new_empty()?;
                        db.save_to_file(&self.metadata_dir_path)?;
                        db
                    }
                    else {return Err(e);}
                }
            }
        );
        Ok(())
    }

    /// Function reads whole column data and calculates either mean of its 
    /// values or counts how many of each character there is 
    pub fn read_col_data(&self, col_name: &str) -> Result<(), String>
    {
        if self.db_meta.is_none()
        {
            return Err(format!("read_col_data - data base is not initialized, db_meta is None"));
        }
        if !self.db_meta.as_ref().unwrap().col_names_idxs().contains_key(col_name)
        {
            return Err(format!("read_col_data - given col name: {} is not present in database", col_name));
        }

        let meta = self.db_meta.as_ref().unwrap();
        let file_names = meta.col_files_paths().get(col_name).unwrap();
        let mut f = File::open(file_names.get(0).unwrap()).unwrap();

        if !self.col_data_map.contains_key(col_name)
        {
            // let col_h = ColHeader::read_from_buf(curr_buf_idx, bytes_read, buf)
        }

        Ok(())
    }
    /// This function TAKES OWNERSHIP of **f**: File. <br>
    /// It returns either the same f or a file hook to newly created file
    /// - Function appends bytes_read bytes to a given file
    /// - If given file has to little space, it creates new one while also 
    /// updating metadata and creating new ColHeader object
    /// - we expect to write chunk_size bytes all the time except last time
    /// 
    /// - db manager first checks if col exists, 
    ///     - if it does db will open its file and pass handler to this function
    ///     - if it doesnt it creates file and passes handler here
    pub fn save_data_chunk_to_file(
        &mut self,
        mut f: File,            
        col_header: &mut ColHeader,
        bytes_read: usize,
        buf: &[u8; CHUNK_SIZE_BYTES] 
    ) ->Result<File, io_err>
    {
        match col_header.increase_data_size(bytes_read as u32)
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
                col_header.modify_data_size_in_file(&mut f)?;

                // We no longer need old col_header, we will write to a new file
                *col_header = col_header.create_next()?;
                let (file_path, new_f) = col_header.save_to_file(&self.col_dir_path)?;

                // Now we need to update our metadata
                self.db_meta
                    .as_mut()
                    .unwrap()
                    .append_new_file_path(col_header.col_name(), file_path)?;

                // And now we recursively invoke this function, since now we 
                // will go into OK branch
                return self.save_data_chunk_to_file(
                    new_f, 
                    col_header, 
                    bytes_read, 
                    buf);
            }
        }
    }
    // pub fn save_data_to_column(col_name: &String, )
}