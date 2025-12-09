use std::collections::{HashMap, HashSet};
use tokio::task::JoinHandle;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use crate::db::constants::{MAINTENANCE_WORKER_ID, MAINTENANCE_WORKER_QUERY_ID};
use crate::db::errors::DbError;
use crate::db::manager::messages::{DbCmd, DbMaintenanceMsg, DbWorkerMsg, QueryData};
use crate::db::manager::db_workers::maintenance_worker::MaintenanceWorker;
use crate::db::manager::db_workers::query_worker::QueryWorker;

use log::{info, warn, debug, error};

pub struct WorkersManager
{
    query_workers: HashMap<usize, QueryWorkerHandler>,
    available_workers: HashSet<usize>,
    maintenance_worker: MaintenanceWorker,
}

impl WorkersManager
{
    pub fn new(
        db_data_dir_path: &str,
        n_workers: usize,
        tx_to_db: UnboundedSender<DbCmd>
    ) -> WorkersManager
    {
        debug!("Maintenance Worker '{}' spawned", MAINTENANCE_WORKER_ID);
        let maintenance_worker = MaintenanceWorker::spawn(
            MAINTENANCE_WORKER_ID,
            db_data_dir_path, 
            tx_to_db.clone()
        );
        let mut query_workers: HashMap<usize, QueryWorkerHandler> = HashMap::new();
        let mut available_workers: HashSet<usize> = HashSet::new();

        for worker_id in 1..=n_workers
        {
            let (tx_to_worker, rx) = unbounded_channel::<DbWorkerMsg>();

            let mut boxed_query_worker = Box::new(QueryWorker::new(
                worker_id, 
                tx_to_db.clone(), 
                rx
            ));

            let handle = tokio::spawn(async move {
                boxed_query_worker.run().await
            });
            debug!("DbWorker '{}' spawned", worker_id);

            let worker_handler = QueryWorkerHandler::new(worker_id, tx_to_worker, handle);

            available_workers.insert(worker_id);
            query_workers.insert(worker_id, worker_handler);
        }

        WorkersManager { query_workers, available_workers, maintenance_worker}
    }

    pub fn execute_query(
        &mut self,
        query: QueryData
    ) -> Result<(), DbError>
    {
        let worker_id = match self.take_available_worker() {
            Some(id) => id,
            None => return Err(DbError::NotFound(format!("WorkersManager::execute_query: There is no available worker")))
        };

        // This should never return error if we coded freeing workers correctly
        let worker = self.query_workers
                        .get(&worker_id)
                        .ok_or_else(|| DbError::NotFound(
                            format!("WorkerrsManager::execute_query: db worker with id: '{}' does not exist", worker_id)
                        ))?;

        worker.send_msg(DbWorkerMsg::ExecQuery(worker_id, query))?;

        Ok(())
    }

    pub fn free_worker(&mut self, worker_id: usize) -> Result<(), DbError>
    {
        if self.query_workers.contains_key(&worker_id)
        {
            self.available_workers.insert(worker_id);
            return Ok(());
        }
        Err(DbError::InternalDbError(format!("WorkersManager::free_worker - worker with id: {} does not exist", worker_id)))
    }

    pub fn is_any_worker_available(&self) -> bool
    {
        !self.available_workers.is_empty()
    }

    pub fn notify_maintenance_worker(
        &self, 
        msg: DbMaintenanceMsg
    ) -> Result<(), DbError> 
    {
        self.maintenance_worker.send_msg(msg)
    }

    /// Method **consumes self** !!!
    pub async fn shutdown(self) -> Result<(), DbError> 
    {
        self.notify_all_workers_about_shutdown()?;
        // TODO: we probably should store all errors on vec and create multiple error msg from it
        let _ = self.maintenance_worker.await_task().await;

        for (_, handler) in self.query_workers
        {
            let _ = handler.handle.await;
        }

        Ok(())
    }

    // ########################################################################
    // ########################### PRIVATE METHODS ############################
    // ########################################################################
    fn notify_all_workers_about_shutdown(&self) -> Result<(), DbError>
    {
        self.notify_maintenance_worker(DbMaintenanceMsg::Shutdown)?;

        for (_, handler) in &self.query_workers
        {
            handler.send_msg(DbWorkerMsg::Shutdown)?;
        }
        Ok(())
    }

    fn take_available_worker(&mut self) -> Option<usize>
    {
        if let Some(&id) = self.available_workers.iter().next() 
        {
            self.available_workers.remove(&id);
            Some(id)
        } 
        else 
        {
            None
        }
    }

}

struct QueryWorkerHandler
{
    id: usize,
    tx: UnboundedSender<DbWorkerMsg>,
    handle: JoinHandle<()>
}

impl QueryWorkerHandler
{
    pub fn new(
        id: usize, 
        tx: UnboundedSender<DbWorkerMsg>, 
        handle: JoinHandle<(

        )>) -> QueryWorkerHandler
    {
        QueryWorkerHandler {id, tx, handle}
    }

    pub fn send_msg(&self, msg: DbWorkerMsg) -> Result<(), DbError>
    { 
        self.tx
            .send(msg)
            .map_err(|e| DbError::InternalDbError(
                format!("QueryWorker::send_msg failed to send message to task: {}", e)
            ))
    }
}
