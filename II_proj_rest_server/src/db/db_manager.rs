use crate::db::storage::col_data::ColData;
use crate::db::storage::col_header::ColHeader;
use crate::db::storage::metadata::{DbMetadata, TableId};
use crate::db::constants::{LogicalColType, BATCH_SIZE, METADATA_FILE_PATH, MAX_ALLOWED_METADATA_CHANGES};
use crate::schemas::query::ShallowQuery;
use crate::schemas::table::{TableSchema, ShallowTable};
use crate::db::csv_reader;
use crate::db::errors::DbError;
use uuid::Uuid;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use std::io::ErrorKind as err_kind;
use std::collections::HashMap;
use std::fs::File;

pub enum TaskMessage
{
    SaveMetadata(DbMetadata),
    Shutdown
}


pub struct DbManager
{
    db_meta: Option<DbMetadata>,
    queries_history: Vec<ShallowQuery>,
    metadata_dir_path: String, 
    data_dir_path: String,
    nbr_of_metadata_changes: u16,
    tx_metadata_saver: UnboundedSender<TaskMessage>,
}

impl DbManager
{
    pub fn new(db_data_dir: &str, tx: UnboundedSender<TaskMessage>) -> DbManager
    {
        DbManager{
            db_meta: None,
            queries_history: Vec::new(),
            metadata_dir_path: String::from(METADATA_FILE_PATH),
            data_dir_path: String::from(db_data_dir),
            nbr_of_metadata_changes: 0,
            tx_metadata_saver: tx
        }
    }

    pub async fn init_db(&mut self) -> Result<(), DbError>
    {
        // To start db, db metadata file must be present
        let metadata_dir = &self.metadata_dir_path;
        let data_dir = &self.data_dir_path;
        self.db_meta = Some(
            match DbMetadata::read_from_file(metadata_dir).await
            {
                Ok(meta) => meta,
                Err(DbError::IoError(ref io_err)) 
                    if io_err.kind() == err_kind::NotFound => {
                        let db = DbMetadata::new_empty(
                            metadata_dir, 
                            data_dir,
                        )?;

                        db.save_to_file().await?;

                        db
                    },
                Err(e) => return Err(e)
            }
        );
        Ok(())
    }

    pub fn get_tables(&self) -> Result<Vec<ShallowTable>, DbError>
    {
        if let Some(meta) = &self.db_meta
        {
            return Ok(meta.get_tables());
        }

        Err(DbError::NotFound(format!("DbManager::get_tables: database was not initialized")))
    }

    pub fn get_table_details(
        &self, 
        table_id: &Uuid
    ) -> Result<TableSchema, DbError>
    {
        if let Some(meta) = &self.db_meta
        {
            return meta.get_table_details(table_id);
        }

        Err(DbError::NotFound(format!("DbManager::get_table_details: database was not initialized")))
    }

    pub async fn put_table(
        &mut self, 
        schema: &TableSchema
    ) -> Result<Uuid, DbError> 
    {
        let db_meta_clone: DbMetadata;
        let table_id: Uuid;

        // Scope needed so that we DROP MUTABLE REF to db_meta, to be able to 
        // use self.send_task_msg
        {
            let db_meta = self.db_meta
                    .as_mut()
                    .ok_or_else(|| DbError::NotFound("DbManager::put_table: database was not initialized".to_string()
            ))?;
            table_id = db_meta.put_table(schema).await?;
            db_meta_clone = db_meta.clone();
        }

        self.nbr_of_metadata_changes += 1;

        if self.nbr_of_metadata_changes >= MAX_ALLOWED_METADATA_CHANGES
        {
            self.nbr_of_metadata_changes = 0;
            self.send_task_msg(TaskMessage::SaveMetadata(db_meta_clone))?;
        }
        Ok(table_id)
    }

    pub fn save_metadata(&self) -> Result<(), DbError>
    {
        if let Some(meta) = &self.db_meta
        {
            self.send_task_msg(TaskMessage::SaveMetadata(meta.clone()))?;
            return Ok(());
        }

        Err(DbError::NotFound(format!("DbManager::put_table: database was not initialized")))
    }

    pub fn shutdown(&self) -> Result<(), DbError>
    {
        self.send_task_msg(TaskMessage::Shutdown)
    }

    fn send_task_msg(&self, msg: TaskMessage) -> Result<(), DbError> 
    {
        self.tx_metadata_saver
            .send(msg)
            .map_err(|e| DbError::InternalDbError(
                format!("DbManager::put_table: failed to send save message: {}", e)
            ))
    }
    // Currently naive implementation just to create some files for our db <br>
    // !!!!!!!! <br> 
    // !!!!!!!! NOT STREAMING, so probably huge csv files will give error <br>
    // !!!!!!!! 
    // pub fn init_from_csv(
    //     &mut self, 
    //     csv_path: &str, 
    //     delim: u8
    // ) -> Result<(), DbError>
    // {
    //     let (metadata, col_data) = 
    //             DbManager::_get_data_and_metadata_from_csv(csv_path, delim)?;
    //     let col_names = metadata.col_names();
    //     let col_types = metadata.col_types();
    //     let n_cols = col_names.len();

    //     self._init_storage_maps(n_cols, col_names, col_types);

    //     for (idx, col_data_vec) in col_data.iter().enumerate()
    //     {
    //         let c_type = LogicalColType::from_u8(*col_types.get(idx).unwrap())?;
    //         let c_name = col_names.get(idx).unwrap().clone();

    //         if c_type == LogicalColType::IntType
    //         {
    //             let col_data_storage = self.int_storage_map.get_mut(&c_name).unwrap();

    //             // Parse strings to i64
    //             let mut int_values: Vec<i64> = Vec::new();
    //             for str_val in col_data_vec {
    //                 match str_val.parse::<i64>() {
    //                     Ok(val) => int_values.push(val),
    //                     Err(e) => return Err(
    //                         DbError::Other(
    //                             format!("Failed to parse '{}' as i64: {}", str_val, e)
    //                         ))
    //                 }
    //             }
                
    //             // Process data in BATCH_SIZE chunks 
    //             for chunk in int_values.chunks(BATCH_SIZE) 
    //             {
    //                 col_data_storage.save_to_file(chunk)?; 
    //             }
    //         }
    //         else 
    //         {
    //             let col_data_storage = self.str_storage_map.get_mut(&c_name).unwrap();

    //             for chunk in col_data_vec.chunks(BATCH_SIZE)
    //             {
    //                 col_data_storage.save_to_file(chunk)?;
    //             }
    //         }
    //     }

    //     metadata.save_to_file(METADATA_FILE_PATH)?;

    //     self.db_meta = Some(metadata);

    //     Ok(())
    // }

    // Function reads whole column data and calculates either mean of its 
    // values or counts how many of each character there is 
    // pub fn read_col_data(&mut self, col_name: &str) -> Result<usize, DbError>
    // {
    //     let meta = self.db_meta.as_ref()
    //         .ok_or_else(|| DbError::Other(
    //             "read_col_data - database is not initialized, db_meta is None".to_string()
    //         ))?;

    //     if !meta.col_names_idxs().contains_key(col_name)
    //     {
    //         return Err(DbError::InvalidColumnName {
    //             msg: "Column name not present in database".to_string(),
    //             name: col_name.to_string()
    //         });
    //     }

    //     let c_idx = *meta.col_names_idxs()
    //         .get(col_name)
    //         .ok_or_else(|| DbError::Other(
    //             format!("read_col_data - col_names_idx - column '{}' index not found", col_name)
    //         ))?;

    //     let c_type = LogicalColType::from_u8(
    //         *meta.col_types()
    //             .get(c_idx)
    //             .ok_or_else(|| DbError::Other(
    //                 format!("read_col_data - column type at index {} not found", c_idx)
    //             ))?
    //     )?;

    //     let file_path = meta.col_files_paths().get(col_name).unwrap().first().unwrap();

    //     let f = File::open(file_path)?;
    //     let mut n_rows: usize = 0;

    //     if c_type == LogicalColType::IntType
    //     {
    //         let col_data = ColData::<i64>::read_from_file(f)?;
    //         n_rows = col_data.n_rows();

    //         self.int_storage_map.insert(
    //             String::from(col_name),  
    //             col_data
    //         );
    //     }
    //     else 
    //     {
    //         let col_data = ColData::<String>::read_from_file(f)?;
    //         n_rows = col_data.n_rows();

    //         self.str_storage_map.insert(
    //             String::from(col_name),  
    //             col_data
    //         );
    //     }

    //     Ok(n_rows)
    // }

    // pub fn read_all_col_data(&mut self) -> Result<(), DbError>
    // {
    //     let meta = self.db_meta.as_ref()
    //         .ok_or_else(|| DbError::Other(
    //             "read_all_col_data - database is not initialized, db_meta is None".to_string()
    //         ))?;

    //     let column_names = meta.col_names().clone();

    //     for name in column_names
    //     {
    //         if !self.is_row_count_init
    //         {
    //             self.is_row_count_init = true;
    //             self.row_count = self.read_col_data(&name)?;
    //         }
    //         else 
    //         {
    //             let n = self.read_col_data(&name)?;
    //             if self.row_count != n
    //             {
    //                 return Err(DbError::SizeMismatch {
    //                     msg: format!("DbManager::read_all_col_data - column '{}' has different row count", name),
    //                     size_1: n,
    //                     size_2: self.row_count
    //                 });
    //             }
    //         }
    //     }

    //     for (name, data) in &self.int_storage_map
    //     {
    //         println!("{}, avg: {}", name, data.result());
    //     }

    //     for (name, data) in &self.str_storage_map
    //     {

    //         println!("{}, count: {}", name, data.result());
    //     }

    //     Ok(())
    // }

    // ################# PRIVATE FUNCTIONS ######################

    // fn _init_storage_maps(
    //     &mut self, 
    //     n_cols: usize, 
    //     col_names: &Vec<String>,
    //     col_types: &Vec<u8>  
    // )
    // {
    //     // In metadata we have all information about columns so we can populate
    //     // hash map that will store column_name : ColData objects which handle
    //     // deserialization and serialization of data
    //     for idx in 0..n_cols
    //     {
    //         let col_name = col_names.get(idx).unwrap().clone();
    //         let col_type = LogicalColType
    //                     ::from_u8(*col_types.get(idx).unwrap())
    //                     .unwrap();
    //         let col_h = ColHeader
    //                     ::new_empty(col_type, col_name.clone())
    //                     .unwrap();

    //         if col_type == LogicalColType::IntType
    //         {
    //             let col_d: ColData<i64> = ColData::new(col_h).unwrap();
    //             self.int_storage_map.insert(col_name, col_d);

    //         }
    //         else 
    //         {
    //             let col_d: ColData<String> = ColData::new(col_h).unwrap();
    //             self.str_storage_map.insert(col_name, col_d);
    //         }
    //     }
    // }


    // fn _get_data_and_metadata_from_csv(
    //     csv_path: &str, 
    //     delim: u8
    // ) -> Result<(DbMetadata, Vec<Vec<String>>), DbError>
    // {
    //     if delim != b',' && delim != b'\t'
    //     {
    //         return Err(DbError::Other(
    //             "We support only csv (comma) or tsv (tab) delimiters".to_string()
    //         ));
    //     }

    //     let (types, names, col_data) = csv_reader::read_csv(csv_path, delim);

    //     let metadata = DbMetadata::new(types, names)?;

    //     metadata.save_to_file(METADATA_FILE_PATH)?;

    //     Ok((metadata, col_data))
    // }
}