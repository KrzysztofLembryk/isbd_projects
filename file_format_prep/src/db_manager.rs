use crate::storage::metadata_structs::DbMetadata;
use crate::constants::{METADATA_FILE_PATH, DB_DATA_DIR};
use std::io::Error as io_err;
use std::io::ErrorKind as err_kind;


struct DbManager
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

    // pub fn save_data_to_column(col_name: &String, )
}