use crate::storage::column_structs::ColHeader;
use crate::storage::metadata_structs::DbMetadata;
use crate::constants::{METADATA_FILE_PATH, DB_DATA_DIR};
use std::fs::File;
use std::io::{Error as io_err, Write};
use std::io::ErrorKind as err_kind;
use std::io::{Seek, SeekFrom};

pub struct DbManager
{
    db_meta: Option<DbMetadata>
}

impl DbManager
{
    pub fn new() -> DbManager
    {
        DbManager{
            db_meta: None
        }
    }

    pub fn start_db(&mut self) -> Result<(), io_err>
    {
        // To start db, db metadata file must be present
        self.db_meta = Some(
            match DbMetadata::read_from_file(METADATA_FILE_PATH)
            {
                Ok(meta) => meta,
                Err(e) => {
                    // if there is no metadata file, we create a new one
                    if e.kind() == err_kind::NotFound 
                    {
                        let db = DbMetadata::new_empty()?;
                        db.save_to_file(METADATA_FILE_PATH)?;
                        db
                    }
                    else {return Err(e);}
                }
            }
        );
        Ok(())
    }

    /// This function TAKES OWNERSHIP of **f**: File. <br>
    /// It returns either the same f or a file hook to newly created file
    /// - Function appends bytes_read bytes to a given column file
    /// - If given file has to little space, it creates new one while also 
    /// updating metadata
    /// - we expect to write chunk_size bytes all the time except last time
    pub fn save_data_chunk_to_file(
        &mut self,
        mut f: File, // db manager will first check if col exists and then
                      // will open file will run this func to save data 
        col_header: &mut ColHeader,
        bytes_read: usize,
        buf: &[u8] // buf max len is CHUNK SIZE
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
                // file, but before that we write updated col_header to file
                col_header.modify_data_size_in_file(&mut f)?;

                // We no longer need old col_header since now we will write 
                // to a new file
                *col_header = col_header.create_next()?;
                let (file_path, new_f) = col_header.save_to_file()?;

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