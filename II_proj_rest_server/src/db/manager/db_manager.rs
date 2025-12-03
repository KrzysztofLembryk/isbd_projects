use crate::db::storage::metadata::{DbMetadata, TableMetadata, TableId};
use crate::db::constants::{MAX_DB_WORKERS};
use crate::schemas::query::{AllowedQuery, CopyQuery, Query, QueryStatus, SelectQuery, ShallowQuery};
use crate::schemas::table::{TableSchema, ShallowTable};
use crate::db::errors::DbError;
use crate::db::manager::messages::{DbMaintenanceMsg, ResMsg, DbCmd};
use crate::db::manager::workers_manager::WorkersManager;

use uuid::Uuid;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use std::io::ErrorKind as err_kind;
use std::collections::HashMap;
use actix_web::{web};

pub type SharedDbManager = web::Data<tokio::sync::RwLock<DbManager>>;


// Separate Db task running in the background
// When wanting table data endpoint sends msg to DB task and awaits response
// When wanting quries details -||-
// When submitting new query endpoint sends message, DB task creates task that
// will handle this query and will wait for its execution (query task will send messages to db so that db can change query status)
// Basically we will do an executor system 

struct DbPaths
{
    metadata_file_path: String, 
    data_dir_path: String,
}

impl DbPaths
{
    pub fn new(
        metadata_file_path: &str, 
        data_dir_path: &str,
    ) -> DbPaths
    {
        DbPaths 
        { 
            metadata_file_path: String::from(metadata_file_path), data_dir_path: String::from(data_dir_path)
        }
    }
}

struct QueryStore
{
    queries: HashMap<Uuid, Query>,
    query_queue: Vec<Uuid>, // stores order of queries to be executed
}

impl QueryStore
{
    pub fn new() -> QueryStore
    {
        QueryStore { queries: HashMap::new(), query_queue: Vec::new() }
    }

    pub fn queries(&self) -> &HashMap<Uuid, Query>
    {
        &self.queries
    }

    pub fn push_query(&mut self, q: Query) -> Result<(), DbError>
    {
        let q_id = q.id().clone();

        if self.queries.contains_key(&q_id)
        {
            return Err(DbError::Other(format!("QueryStore::push_query: Query with id: {} already exists", q_id)));
        }
        // TODO: add check if query already exists
        // and if we have enough space in vector
        self.queries.insert(q_id.clone(), q);
        self.query_queue.push(q_id);    

        Ok(())
    }

    pub fn update_query_status(
        &mut self, 
        q_id: &Uuid, 
        new_status: QueryStatus
    ) -> Result<(), DbError>
    {
        if let Some(q) = self.queries.get_mut(q_id)
        {
            q.update_status(new_status);
            return Ok(());
        }
        Err(DbError::NotFound(format!("QueryStore::update_query_status: query with id: '{}'", q_id)))
    }
}

pub struct DbManager
{
    db_meta: Option<DbMetadata>,
    query_store: QueryStore,
    paths: DbPaths,
    workers_manager: Option<WorkersManager>,
    tx_server_channels: HashMap<Uuid, UnboundedSender<ResMsg>>
}

impl DbManager
{
    pub async fn new(
        db_data_dir_path: &str, 
        metadata_file_path: &str
    ) -> Result<DbManager, DbError>
    {
        println!("metadata file path: {}, db_data_dir_path: {}", metadata_file_path, db_data_dir_path);
        let mut db_manager = DbManager{
            db_meta: None,
            query_store: QueryStore::new(),
            paths: DbPaths::new(metadata_file_path, db_data_dir_path),
            workers_manager: None, 
            tx_server_channels: HashMap::new(),
        };

        db_manager.init_metadata().await?;

        Ok(db_manager)
    }

    pub fn init_worker_manager(
        &mut self,
        tx_to_db: UnboundedSender<DbCmd>
    )
    {
        self.workers_manager = Some(WorkersManager::new(
            &self.paths.data_dir_path, 
            MAX_DB_WORKERS, 
            tx_to_db)
        );
    }

    // ########################################################################
    // ########################## TABLES HANDLERS #############################
    // ########################################################################

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

    pub fn mark_table_to_delete(
        &mut self, table_id: &Uuid) -> Result<(), DbError>
    {
        let db_meta = self.db_meta
                .as_mut()
                .ok_or_else(|| DbError::NotFound("DbManager::mark_table_to_delete: database was not initialized".to_string()
        ))?;

        db_meta.mark_table_to_delete(table_id)?;

        Ok(())
    }

    pub fn delete_table(
        &mut self,
        table_id: &Uuid
    ) -> Result<TableMetadata, DbError>
    {
        let db_meta = self.db_meta
                .as_mut()
                .ok_or_else(|| DbError::NotFound("DbManager::delete_table: database was not initialized".to_string()
        ))?;

        db_meta.delete_table(table_id)
    }

    pub fn put_table(
        &mut self, 
        schema: &TableSchema
    ) -> Result<Uuid, DbError> 
    {
        let db_meta = self.db_meta
                .as_mut()
                .ok_or_else(|| DbError::NotFound("DbManager::put_table: database was not initialized".to_string()
        ))?;
        let table_id = db_meta.put_table(schema)?;

        if db_meta.is_enough_changes()
        {
            let db_meta_clone = db_meta.clone();

            db_meta.reset_changes();
            self.notify_db_maintenance_background_task(
                DbMaintenanceMsg::SaveMetadata(db_meta_clone)
            )?;
        }

        Ok(table_id)
    }

    // ########################################################################
    // ########################## QUERIES HANDLERS ############################
    // ########################################################################

    pub fn get_queries(&self) -> Result<Vec<ShallowQuery>, DbError>
    {
        // TODO: We should also check if db is initialized here
        Ok(self.query_store
            .queries()
            .iter()
            .map(|(q_id, q_data)| ShallowQuery::new(*q_id, q_data.status()))
            .collect())
    }

    pub fn get_query_details(&self, query_id: &Uuid) -> Result<Query, DbError>
    {
        if let Some(q) = self.query_store.queries().get(query_id)
        {
            return Ok(q.clone());
        }
        Err(DbError::NotFound(format!("Query with id: {}, not found in db.", query_id)))
    }

    pub fn post_query(&mut self, query: AllowedQuery) -> Result<Uuid, DbError>
    {
        // Will add query to db_manager query map
        // Will spawn task that handles query
        match query
        {
            AllowedQuery::COPY_Q(q) => self.handle_copy_query(&q),
            AllowedQuery::SELECT_Q(q) => self.handle_select_query(&q),
        }
        todo!("implement post_query")
    }

    fn handle_copy_query(&mut self, query: &CopyQuery)
    {

    }
    fn handle_select_query(&mut self, query: &SelectQuery)
    {

    }

    // ########################################################################
    // ################## SERVER CONNECTIONS COMMUNICATION ####################
    // ########################################################################
    pub fn register(
        &mut self, 
        connection_id: &Uuid, 
        tx: UnboundedSender<ResMsg>
    )
    {
        println!("Registering: {}", connection_id);
        self.tx_server_channels.insert(connection_id.clone(), tx);
    }

    pub fn unregister(&mut self, connection_id: &Uuid)
    {
        println!("Unregistering: {}", connection_id);
        self.tx_server_channels.remove(connection_id);
    }

    pub fn send_result(
        &self, 
        id: &Uuid, 
        msg: ResMsg
    ) -> Result<(), DbError>
    {
        if let Some(tx) = self.tx_server_channels.get(id)
        {
            match tx.send(msg)
            {
                Ok(_) => return Ok(()),
                Err(e) => return Err(DbError::Other(format!("DbManager::send_result: error '{}'", e)))
            };
        }
        Err(DbError::NotFound(format!("channel with id: {} ", id)))
    }

    // ########################################################################
    // ############################# DB SHUTDOWN ##############################
    // ########################################################################
    pub async fn shutdown(self) -> Result<(), DbError>
    {
        self.save_metadata()?;
        self.perform_shutdown().await?;
        // TODO: await for tasks
        Ok(())
    }

    // ########################################################################
    // ################## DB MAINTENANCE TASK COMMUNICATION ###################
    // ########################################################################
    
    pub fn schedule_table_deletion(
        &self, 
        t_meta: TableMetadata
    ) -> Result<(), DbError> 
    {
        self.notify_db_maintenance_background_task(
            DbMaintenanceMsg::DeleteTable(t_meta)
        )
    }

    fn save_metadata(&self) -> Result<(), DbError>
    {
        if let Some(meta) = &self.db_meta
        {
            self.notify_db_maintenance_background_task(DbMaintenanceMsg::SaveMetadata(meta.clone()))?;
            return Ok(());
        }

        Err(DbError::NotFound(format!("DbManager::put_table: database was not initialized")))
    }

    async fn perform_shutdown(self) -> Result<(), DbError>
    {
        self.notify_db_maintenance_background_task(DbMaintenanceMsg::Shutdown)?;
        match self.workers_manager.await_task().await
        {
            Ok(()) => Ok(()),
            Err(e) => Err(DbError::Other(format!("error: {}", e)))
        }
    }

    fn notify_db_maintenance_background_task(
        &self, 
        msg: DbMaintenanceMsg
    ) -> Result<(), DbError> 
    {
        self.workers_manager.send_msg(msg)
    }

    async fn init_metadata(&mut self) -> Result<(), DbError>
    {
        println!("INITING METADATA");
        // To start db, db metadata file must be present
        let metadata_path = &self.paths.metadata_file_path;
        let data_dir = &self.paths.data_dir_path;

        println!("INIT_METADATA: metadata file path: {}, db_data_dir_path: {}", metadata_path, data_dir);
        self.db_meta = Some(
            match DbMetadata::read_from_file(metadata_path).await
            {
                Ok(meta) => meta,
                Err(DbError::IoError(ref io_err)) 
                    if io_err.kind() == err_kind::NotFound => {
                        let db = DbMetadata::new_empty(
                            metadata_path, 
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