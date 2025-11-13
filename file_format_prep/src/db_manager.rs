use crate::storage::col_data::ColData;
use crate::storage::col_header::ColHeader;
use crate::storage::metadata_structs::DbMetadata;
use crate::constants::{AllowedColTypes, BATCH_SIZE, CHUNK_SIZE_BYTES, DB_DATA_DIR, METADATA_FILE_PATH};
use crate::csv_reader;

use std::collections::HashMap;
use std::fs::File;
use std::io::{Error as io_err, Write};
use std::io::ErrorKind as err_kind;
use std::io::{Seek, SeekFrom};

pub struct DbManager
{
    db_meta: Option<DbMetadata>,
    str_cols_map: HashMap<String, ColData<String>>,
    int_cols_map: HashMap<String, ColData<i64>>,
    metadata_dir_path: String,
    col_dir_path: String,
    row_count: usize, 
    is_row_count_init: bool,
}

impl DbManager
{
    pub fn new(cols_dir_path: &str) -> DbManager
    {
        DbManager{
            db_meta: None,
            str_cols_map: HashMap::new(),
            int_cols_map: HashMap::new(),
            metadata_dir_path: String::from(METADATA_FILE_PATH),
            col_dir_path: String::from(cols_dir_path),
            row_count: 0,
            is_row_count_init: false,
        }
    }

    pub fn init_db(&mut self) -> Result<(), io_err>
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

    /// Currently naive implementation just to create some files for our db <br>
    /// !!!!!!!! <br> 
    /// !!!!!!!! NOT STREAMING, so probably huge csv files will give error <br>
    /// !!!!!!!! 
    pub fn init_from_csv(&mut self, csv_path: &str, delim: u8) -> Result<(), String>
    {
        if delim != b',' && delim != b'\t'
        {
            return Err(format!("We support either csv or tsv"));
        }

        let (types, names, col_data) = csv_reader::read_csv(csv_path, delim);

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
        let n_cols = col_names.len();

        // In metadata we have all information about columns so we can populate
        // hash map that will store column_name : ColData objects which handle
        // deserialization and serialization of data
        for idx in 0..n_cols
        {
            let col_name = col_names.get(idx).unwrap().clone();
            let col_type = AllowedColTypes::from_u8(*col_types.get(idx).unwrap()).unwrap();
            let col_h = ColHeader::new_empty(col_type, col_name.clone()).unwrap();
            if col_type == AllowedColTypes::IntType
            {
                let col_d: ColData<i64> = ColData::new(col_h).unwrap();
                self.int_cols_map.insert(col_name, col_d);

            }
            else 
            {
                println!("String col not impl yet");
            }
        }

        for (idx, col_data_vec) in col_data.iter().enumerate()
        {
            
            let c_type = AllowedColTypes::from_u8(*col_types.get(idx).unwrap())?;
            let c_name = col_names.get(idx).unwrap().clone();

            if c_type == AllowedColTypes::IntType
            {
                let col_data_storage = self.int_cols_map.get_mut(&c_name).unwrap();

                // Parse strings to i64
                let mut int_values: Vec<i64> = Vec::new();
                for str_val in col_data_vec {
                    match str_val.parse::<i64>() {
                        Ok(val) => int_values.push(val),
                        Err(e) => return Err(format!("Failed to parse '{}' as i64: {}", str_val, e))
                    }
                }
                
                // Process data in BATCH_SIZE chunks 
                for chunk in int_values.chunks(BATCH_SIZE) {
                    col_data_storage.save_to_file(chunk);                  
                }
                // FOR TESTING WE SAVE ONLY ONE FILE
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


    /// Function reads whole column data and calculates either mean of its 
    /// values or counts how many of each character there is 
    pub fn read_col_data(&mut self, col_name: &str) -> Result<usize, String>
    {
        let meta = self.db_meta.as_ref();
        if meta.is_none()
        {
            return Err(format!("read_col_data - data base is not initialized, db_meta is None"));
        }

        let meta = meta.unwrap();
        if !meta.col_names_idxs().contains_key(col_name)
        {
            return Err(format!("read_col_data - given col name: {} is not present in database", col_name));
        }

        let c_idx = *meta.col_names_idxs().get(col_name).unwrap();
        let c_type = AllowedColTypes::from_u8(*meta.col_types().get(c_idx).unwrap()).unwrap();

        let file_path = meta.col_files_paths().get(col_name).unwrap().first().unwrap();

        let f = File::open(file_path).unwrap();
        let mut n_rows: usize = 0;

        if c_type == AllowedColTypes::IntType
        {
            let col_data = ColData::<i64>::read_from_file(f);
            n_rows = col_data.n_rows();

            self.int_cols_map.insert(
                String::from(col_name),  
                col_data
            );
        }
        else 
        {
            println!("String not implemented yet");
        }

        Ok(n_rows)
    }

    pub fn read_all_col_data(&mut self) -> Result<(), String>
    {
        let meta = self.db_meta.as_ref();
        if meta.is_none()
        {
            return Err(format!("read_all_col_data - data base is not initialized, db_meta is None"));
        }
        let meta = meta.unwrap();

        let column_names = meta.col_names().clone();


        for name in column_names
        {
            if !self.is_row_count_init
            {
                self.is_row_count_init = true;
                self.row_count = self.read_col_data(&name)?;
            }
            else 
            {
                let n = self.read_col_data(&name)?;
                if self.row_count != n
                {
                    return Err(format!("read_all_col_data - column: '{}', has different row count: '{}' than the others: '{}'", name, n, self.row_count));
                }
            }
        }

        for (name, data) in &self.int_cols_map
        {
            println!("{}, avg: {}", name, data.result());
        }

        for (name, data) in &self.str_cols_map
        {

            println!("{}, count: {}", name, data.result());
        }

        Ok(())
    }
}