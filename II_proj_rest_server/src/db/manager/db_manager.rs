use crate::db::storage::metadata::{DbMetadata, TableMetadata};
use crate::db::constants::{MAX_DB_WORKERS};
use crate::schemas::error::MultipleProblemsError;
use crate::schemas::query::{AllowedQuery, CopyQuery, Query, QueryResult, QueryStatus, QueryType, SelectQuery, ShallowQuery};
use crate::schemas::table::{TableSchema, ShallowTable};
use crate::db::errors::DbError;
use crate::db::manager::messages::{DbCmd, DbMaintenanceMsg, DbWorkerMsg, QueryCompletionMsg, QueryFailureMsg, ResMsg, WorkerMsgRes};
use crate::db::manager::db_workers::workers_manager::WorkersManager;
use crate::db::manager::query_store::QueryStore;

use uuid::Uuid;
use tokio::sync::mpsc::{UnboundedSender};
use std::io::ErrorKind as err_kind;
use std::collections::HashMap;
use log::{info, warn, debug, error};

#[cfg(test)]
#[path = "../tests/manager/test_db_manager.rs"]
mod test_db_manager;

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

pub struct DbManager
{
    db_meta: Option<DbMetadata>,
    query_store: QueryStore,
    paths: DbPaths,
    workers_manager: WorkersManager,
    tx_server_channels: HashMap<Uuid, UnboundedSender<ResMsg>>
}

impl DbManager
{
    pub async fn new(
        tx_to_db: UnboundedSender<DbCmd>,
        db_data_dir_path: &str, 
        metadata_file_path: &str,
        nbr_of_db_workers: usize
    ) -> Result<DbManager, DbError>
    {
        let mut db_manager = DbManager{
            db_meta: None,
            query_store: QueryStore::new(),
            paths: DbPaths::new(metadata_file_path, db_data_dir_path),
            workers_manager: WorkersManager::new(
                db_data_dir_path, nbr_of_db_workers, tx_to_db), 
            tx_server_channels: HashMap::new(),
        };

        // TODO: this should be inside DbMangaer, thus db_meta shouldnt be Option
        db_manager.init_metadata().await?;

        Ok(db_manager)
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

        db_meta.mark_table_for_deletion(table_id)?;

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
            self.workers_manager.notify_maintenance_worker(
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
        let db_meta = self.db_meta
                .as_mut()
                .ok_or_else(|| DbError::NotFound("DbManager::post_query: database was not initialized".to_string()
        ))?;

        // Function checks if query is for table that exists 
        // if it is it increases nbr of queries operating on this table
        // if it isn't we mark this query as FAILED and store this query 
        match db_meta.increase_nbr_of_queries_operating_on_table(&query)
        {
            Ok(_) => (),
            Err(e) => {
                let mut new_query = Query::new(
                    query
                );
                new_query.update_status(QueryStatus::FAILED);
                self.query_store.insert_query(new_query)?;

                return Err(e);
            }
        }

        // Provided query is authorized so we create new Query from it with 
        // Status CREATED  
        let new_query = Query::new(
            query
        );
        let new_query_id = new_query.id().clone();

        // We need to remember all queries so we add it to query store
        self.query_store.insert_query(new_query)?;
        // Query is correct thus we schedule it for execution
        self.query_store.schedule_for_execution(&new_query_id);

        self.execute_next_query()?;

        // If no worker is available we just return query id, post was 
        // successful, and query is scheduled for execution
        return Ok(new_query_id);
    }

    pub fn get_query_result(
        &mut self, 
        query_id: &Uuid, 
        row_limit: usize, 
        do_flush: bool
    ) -> Result<QueryResult, DbError>
    {
        let q_res = self.query_store.get_query_result(query_id)?;
        let limited_query_res = q_res.get_n_rows(row_limit)?;

        if do_flush
        {
            self.query_store.remove_query_res(query_id);
        }

        return Ok(limited_query_res);
    }

    pub fn get_query_failed(
        &mut self, 
        query_id: &Uuid, 
    ) -> Result<MultipleProblemsError, DbError>
    {
        self.query_store.get_query_failure(query_id)
    }

    pub fn handle_completed_query(
        &mut self, 
        worker_id: usize, 
        q_msg: QueryCompletionMsg
    ) -> Result<(), DbError>
    {
        self.free_db_worker(worker_id)?;

        let db_meta = self.db_meta
                .as_mut()
                .ok_or_else(|| DbError::NotFound("DbManager::handle_completed_query: database was not initialized".to_string()
        ))?;

        // If everything works as intended this should never return error
        db_meta.decrease_nbr_of_queries_operating_on_table(
            Some(&q_msg.table_id()), 
            None
        )?;
        // if we got copy query this will lift the lock
        let is_copy = self.query_store
            .check_if_query_is_copy(&q_msg.query_id())?;
        db_meta.lift_copy_lock_from_table(&q_msg.table_id(), is_copy)?;

        self.query_store.update_query_status(
            &q_msg.query_id(), 
            QueryStatus::COMPLETED
        )?;
        self.store_completed_query(q_msg)?;
        self.execute_next_query()?;

        Ok(())
    }

    pub fn handle_failed_query(
        &mut self,
        worker_id: usize,
        q_msg: QueryFailureMsg
    ) -> Result<(), DbError>
    {
        // We always want to free worker first
        self.free_db_worker(worker_id)?;

        let db_meta = self.db_meta
                .as_mut()
                .ok_or_else(|| DbError::InternalDbError("DbManager::handle_completed_query: database was not initialized".to_string()
        ))?;

        db_meta.decrease_nbr_of_queries_operating_on_table(
            Some(&q_msg.table_id()),
            None,
        )?;

        let is_copy = self.query_store
            .check_if_query_is_copy(&q_msg.query_id())?;
        db_meta.lift_copy_lock_from_table(&q_msg.table_id(), is_copy)?;

        self.query_store.update_query_status(
            &q_msg.query_id(), 
            QueryStatus::FAILED
        )?;

        self.store_failed_query(q_msg);
        self.execute_next_query()?;

        Ok(())
    }

    fn execute_next_query(&mut self) -> Result<bool, DbError>
    {
        // TODO: move code into separate functions
        if self.workers_manager.is_any_worker_available()
        {
            // TODO: We should loop here till we get the same pending_id, so 
            // that copy query does not stop our other queries
            let pending_q_id = match self.query_store.pop_pending_query()
            {
                Some(q) => q,
                None => return Ok(false)
            };
            let table_name = self.query_store.get_query_table_name(&pending_q_id)?;
            let query_type = self.query_store.get_query_type(&pending_q_id)?;

            let db_meta = self.db_meta
                    .as_mut()
                    .ok_or_else(|| DbError::InternalDbError("DbManager::post_query: database was not initialized".to_string()
            ))?;

            if !DbManager::acquire_copy_lock_if_needed(
                &query_type,
                table_name,
                db_meta
            )? 
            {
                // We failed to acquire lock, meaning some copy query is 
                // currently beeing executed on this table, thus we push 
                // this Query back to the end of queue.
                self.query_store.schedule_for_execution(&pending_q_id);
                return Ok(false);
            }

            let pending_q = self.query_store.get_query_mut_ref(pending_q_id)?;

            // DbMeta plans query execution by checking if table for query 
            // exists, and if it does it gives query file paths and column info 
            let query_plan_data = match db_meta.plan_query_execution(pending_q)
            {
                Ok(val) => val,
                Err(e) => {
                    pending_q.update_status(QueryStatus::FAILED);

                    match db_meta.decrease_nbr_of_queries_operating_on_table(
                        None, 
                        Some(pending_q.table_name())
                    )
                    {
                        Ok(_) => return Err(e),
                        Err(second_err) => return Err(DbError::InternalDbError(format!("Error1: {}\nError2: {}", e, second_err)))
                    }
                }
            };

            pending_q.update_status(QueryStatus::RUNNING);

            match self.workers_manager.execute_query(query_plan_data)
            {
                Ok(_) => {
                    return Ok(true)
                },
                Err(e) => {
                    pending_q.update_status(QueryStatus::FAILED);

                    match db_meta.decrease_nbr_of_queries_operating_on_table(
                        None, 
                        Some(pending_q.table_name())
                    )
                    {
                        Ok(_) => return Err(e),
                        Err(second_err) => return Err(DbError::InternalDbError(format!("Error1: {}\nError2: {}", e, second_err)))
                    }
                }
            }
        }
        // Why we return bool? I don't remember
        Ok(false)
    }

    fn acquire_copy_lock_if_needed(
        query_type: &QueryType,
        table_name: &str,
        db_meta: &mut DbMetadata,
    ) -> Result<bool, DbError>
    {
        match query_type
        {
            QueryType::SelectQuery => {
                return Ok(true);
            },
            QueryType::CopyQuery => {
                let table_id = db_meta.get_table_id(table_name)?;

                // function returns true if lock was acquired, false otherwise
                return db_meta.acquire_copy_lock_for_table(&table_id);
            }
        }
    }

    fn store_completed_query(
        &mut self, 
        completed_q: QueryCompletionMsg
    ) -> Result<(), DbError>
    {
        let query_id = completed_q.query_id();
        let table_id = completed_q.table_id();
        let q_res = completed_q.res();

        match q_res
        {
            WorkerMsgRes::SelectRes(s_res) => {
                self.query_store.store_query_result(
                    &query_id,
                    s_res
                );
            },
            WorkerMsgRes::CopyRes(c_res) => {
                // We are not storing copy query results BUT we need to update
                // table columns filepaths
                let db_meta = self.db_meta
                        .as_mut()
                        .ok_or_else(|| DbError::InternalDbError("DbManager::handle_completed_query: database was not initialized".to_string()
                ))?;
                db_meta.append_newly_created_column_files(
                    &table_id, 
                    c_res
                )?;
            },
        }
        return Ok(());
    }

    fn store_failed_query(&mut self, failed_q: QueryFailureMsg)
    {
        self.query_store.store_query_failure(
            &failed_q.query_id(), 
            failed_q.problems()
        );
    }

    fn free_db_worker(&mut self, worker_id: usize) -> Result<(), DbError>
    {
        self.workers_manager.free_worker(worker_id)
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
        debug!("DbTask::register id: {}", connection_id);
        self.tx_server_channels.insert(connection_id.clone(), tx);
    }

    pub fn unregister(&mut self, connection_id: &Uuid)
    {
        debug!("DbTask::unregister id: {}", connection_id);
        self.tx_server_channels.remove(connection_id);
    }

    pub fn send_result(
        &self, 
        conn_id: &Uuid, 
        msg: ResMsg
    ) -> Result<(), DbError>
    {
        if let Some(tx) = self.tx_server_channels.get(conn_id)
        {
            match tx.send(msg)
            {
                Ok(_) => return Ok(()),
                Err(e) => return Err(DbError::InternalDbError(format!("DbManager::send_result: error '{}'", e)))
            };
        }
        Err(DbError::NotFound(format!("channel with id: {} ", conn_id)))
    }

    // ########################################################################
    // ############################# DB SHUTDOWN ##############################
    // ########################################################################
    pub async fn shutdown(self) -> Result<(), DbError>
    {
        self.save_metadata()?;
        self.perform_shutdown().await?;
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
        return self.workers_manager.notify_maintenance_worker(
            DbMaintenanceMsg::DeleteTable(t_meta)
        );
    }

    fn save_metadata(&self) -> Result<(), DbError>
    {
        if let Some(meta) = &self.db_meta
        {
            return self.workers_manager.notify_maintenance_worker(
                DbMaintenanceMsg::SaveMetadata(meta.clone())
            );
        }

        Err(DbError::NotFound(format!("DbManager::put_table: database was not initialized")))
    }

    async fn perform_shutdown(self) -> Result<(), DbError>
    {
        self.workers_manager.shutdown().await
    }

    async fn init_metadata(&mut self) -> Result<(), DbError>
    {
        // To start db, db metadata file must be present
        let metadata_path = &self.paths.metadata_file_path;
        let data_dir = &self.paths.data_dir_path;

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
                Err(e) => return Err(DbError::InternalDbError(format!("DbManager::init_metadata - {}",e)))
            }
        );
        Ok(())
    }

}