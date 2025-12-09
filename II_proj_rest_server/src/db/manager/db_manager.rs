use crate::db::storage::metadata::{DbMetadata, TableMetadata};
use crate::schemas::error::MultipleProblemsError;
use crate::schemas::query::{AllowedQuery, Query, QueryResult, QueryStatus, QueryType, ShallowQuery};
use crate::schemas::table::{TableSchema, ShallowTable};
use crate::db::errors::DbError;
use crate::db::manager::messages::{DbCmd, DbMaintenanceMsg, QueryCompletionMsg, QueryFailureMsg, ResMsg, WorkerMsgRes};
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
    db_meta: DbMetadata,
    query_store: QueryStore,
    _paths: DbPaths,
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
        let db_meta = DbManager::init_metadata(
                                    metadata_file_path, 
                                    db_data_dir_path
                                ).await?;
        let query_store = QueryStore::new();
        let paths = DbPaths::new(metadata_file_path, db_data_dir_path);
        let workers_manager = WorkersManager::new(
                                        db_data_dir_path, 
                                        nbr_of_db_workers, 
                                        tx_to_db
                                    );
        let tx_server_channels: HashMap<Uuid, UnboundedSender<ResMsg>> = HashMap::new();

        let db_manager = DbManager{
            db_meta,
            query_store,
            _paths: paths,
            workers_manager, 
            tx_server_channels,
        };

        Ok(db_manager)
    }

    // ########################################################################
    // ########################## TABLES HANDLERS #############################
    // ########################################################################

    pub fn get_tables(&self) -> Vec<ShallowTable>
    {
        return self.db_meta.get_tables();
    }

    pub fn get_table_details(
        &self, 
        table_id: &Uuid
    ) -> Result<TableSchema, DbError>
    {
        return self.db_meta.get_table_details(table_id);
    }

    fn mark_table_to_delete(
        &mut self, table_id: &Uuid) -> Result<(), DbError>
    {
        info!("Marking Table for Deletion, id: {}", table_id);
        self.db_meta.mark_table_for_deletion(table_id)
    }

    pub fn delete_table(&mut self, table_id: &Uuid) -> Result<(), DbError>
    {
        match self.mark_table_to_delete(&table_id)
        {
            // We couldnt mark table for deletion, it doesnt exist in our db
            // or already is marked
            Err(e) => {
                return Err(e);
            },
            Ok(_) => {
                match self.db_meta.delete_table(&table_id)
                {
                    // No queries running on table, so we can 
                    // schedule table deletion by Maintenance worker
                    Ok(t_meta) => {
                        debug!("DbManager::delete_table scheduling deletion of table: {}", table_id);

                        self.save_metadata_if_enough_changes()?;
                        return self.schedule_table_deletion(t_meta);
                    },
                    // This means we cannot yet delete table, since
                    // there are some queries running on it, BUT it will be
                    // deleted in near future
                    Err(e) => {
                        debug!("DbManager::delete_table - couldnt delete table files yet: {}", e);
                        return Ok(());
                    }
                }
            },
        }
    }

    pub fn put_table(
        &mut self, 
        schema: &TableSchema
    ) -> Result<Uuid, DbError> 
    {
        let db_meta = &mut self.db_meta;
        let table_id = db_meta.put_table(schema)?;

        self.save_metadata_if_enough_changes()?;

        Ok(table_id)
    }

    // ########################################################################
    // ########################## QUERIES HANDLERS ############################
    // ########################################################################

    pub fn get_queries(&self) -> Result<Vec<ShallowQuery>, DbError>
    {
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
        info!("Details for query: '{}' NOT FOUND", query_id);
        Err(DbError::NotFound(format!("Query with id: {}, not found in db.", query_id)))
    }

    pub fn post_query(&mut self, query: AllowedQuery) -> Result<Uuid, DbError>
    {
        let db_meta = &mut self.db_meta;

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
        debug!("DbManager::handle_completed_query: worker: {} completed query: {}", worker_id, q_msg.query_id());

        self.free_db_worker(worker_id)?;

        let db_meta = &mut self.db_meta;
        let table_id = &q_msg.table_id();

        // If everything works as intended this should never return error
        db_meta.decrease_nbr_of_queries_operating_on_table(
            Some(table_id), 
            None
        )?;

        let is_copy = self.query_store.check_if_query_is_copy(&q_msg.query_id())?;

        debug!("Is this query copy: {}", is_copy);

        db_meta.lift_copy_lock_from_table(&q_msg.table_id(), is_copy)?;
        self.query_store.update_query_status(
            &q_msg.query_id(), 
            QueryStatus::COMPLETED
        )?;

        // We need to firstly store query res, since if it is copy query it will
        // change metadata, so it may happen that we have deleted table before
        // making these changes if storing was after can_table_be_deleted
        DbManager::store_completed_query(
            q_msg, 
            db_meta, 
            &mut self.query_store
        )?;

        if db_meta.can_table_be_deleted(table_id)?
        {
            let table_meta = db_meta.get_table_metadata(table_id)?;
            let table_meta = table_meta.clone();

            db_meta.delete_table(table_id)?;
            self.save_metadata_if_enough_changes()?;

            self.workers_manager.notify_maintenance_worker(
                        DbMaintenanceMsg::DeleteTable(table_meta)
                    )?;
        }

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

        let db_meta = &mut self.db_meta;
        let table_id = &q_msg.table_id();

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

        DbManager::store_failed_query(
            q_msg, 
            &mut self.query_store
        );

        if db_meta.can_table_be_deleted(table_id)?
        {
            let table_meta = db_meta.get_table_metadata(table_id)?;
            let table_meta = table_meta.clone();

            db_meta.delete_table(table_id)?;
            self.save_metadata_if_enough_changes()?;

            self.workers_manager.notify_maintenance_worker(
                        DbMaintenanceMsg::DeleteTable(table_meta)
                    )?;
        }

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

            let db_meta = &mut self.db_meta;

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
        completed_q: QueryCompletionMsg,
        db_meta: &mut DbMetadata,
        query_store: &mut QueryStore,
    ) -> Result<(), DbError>
    {
        let query_id = completed_q.query_id();
        let table_id = completed_q.table_id();
        let q_res = completed_q.res();

        match q_res
        {
            WorkerMsgRes::SelectRes(s_res) => {
                query_store.store_query_result(
                    &query_id,
                    s_res
                );
            },
            WorkerMsgRes::CopyRes(c_res) => {
                // We are not storing copy query results BUT we need to update
                // table columns filepaths
                db_meta.append_newly_created_column_files(
                    &table_id, 
                    c_res
                )?;
            },
        }
        return Ok(());
    }

    fn store_failed_query(failed_q: QueryFailureMsg, query_store: &mut QueryStore)
    {
        query_store.store_query_failure(
            &failed_q.query_id(), 
            failed_q.problems()
        );
    }

    fn free_db_worker(&mut self, worker_id: usize) -> Result<(), DbError>
    {
        self.workers_manager.free_worker(worker_id)
    }

    fn save_metadata_if_enough_changes(&mut self) -> Result<(), DbError>
    {
        if self.db_meta.is_enough_changes()
        {
            let db_meta_clone = self.db_meta.clone();

            self.db_meta.reset_changes();
            return self.workers_manager.notify_maintenance_worker(
                DbMaintenanceMsg::SaveMetadata(db_meta_clone)
            );
        }
        return Ok(());
    }
    // ########################################################################
    // ################## SERVER CONNECTIONS COMMUNICATION ####################
    // ########################################################################
    pub fn register_conn(
        &mut self, 
        connection_id: &Uuid, 
        tx: UnboundedSender<ResMsg>
    )
    {
        self.tx_server_channels.insert(connection_id.clone(), tx);
    }

    pub fn unregister_conn(&mut self, connection_id: &Uuid)
    {
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
    
    fn schedule_table_deletion(
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
        return self.workers_manager.notify_maintenance_worker(
            DbMaintenanceMsg::SaveMetadata(self.db_meta.clone())
        );
    }

    async fn perform_shutdown(self) -> Result<(), DbError>
    {
        self.workers_manager.shutdown().await
    }

    async fn init_metadata(
        metadata_path: &str, 
        data_dir: &str
    ) -> Result<DbMetadata, DbError>
    {
        debug!("DbManager::init_metadata");
        // To start db, db metadata file must be present
        return match DbMetadata::read_from_file(metadata_path).await
        {
            Ok(meta) => Ok(meta),
            Err(DbError::IoError(ref io_err)) 
                if io_err.kind() == err_kind::NotFound => {
                    let db = DbMetadata::new_empty(
                        metadata_path, 
                        data_dir,
                    )?;

                    db.save_to_file().await?;

                    Ok(db)
                },
            Err(e) => return Err(DbError::InternalDbError(format!("DbManager::init_metadata - {}",e)))
        };
    }

}