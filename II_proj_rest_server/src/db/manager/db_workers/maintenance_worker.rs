use crate::db::constants::{DB_DATA_DIR, MAINTENANCE_WORKER_QUERY_ID, MAINTENANCE_SAVE_META_TABLE_ID};
use crate::db::manager::db_workers::maintenance_worker;
use crate::db::manager::messages::{DbCmd, DbMaintenanceMsg, DbWorkerMsg, QueryFailureMsg};
use crate::db::errors::DbError;
use crate::schemas::error::MultipleProblemsError;

use std::path::Path;
use uuid::Uuid;
use tokio::{sync::mpsc::{UnboundedSender, unbounded_channel}, task::JoinError};
use log::{info, warn, debug, error};

pub struct MaintenanceWorker
{
    tx: UnboundedSender<DbMaintenanceMsg>,
    handle: tokio::task::JoinHandle<()>,
}

impl MaintenanceWorker
{
    pub fn new(
        tx: UnboundedSender<DbMaintenanceMsg>,
        handle: tokio::task::JoinHandle<()>
    ) -> MaintenanceWorker
    {
        MaintenanceWorker {tx, handle}
    }

    pub fn send_msg(&self, msg: DbMaintenanceMsg) -> Result<(), DbError>
    {
        self.tx
            .send(msg)
            .map_err(|e| DbError::InternalDbError(
                format!("DbMaintenanceTask::send_msg failed to send message to task: {}", e)
            ))
    }
    /// Function spawns new task that saves, received by msg, metadata to file
    /// It returns MetadataSaver that stores task handle and channel transmitter
    pub fn spawn(
        maintenance_worker_id: usize,
        db_data_dir_path: &str, 
        tx_to_db: UnboundedSender<DbCmd>
    ) -> MaintenanceWorker
    {
        let (tx, mut rx) = unbounded_channel::<DbMaintenanceMsg>();
        let path = String::from(db_data_dir_path);

        let saver_task_handle = tokio::spawn(async move {
                let db_data_dir_path = path.clone();
                loop  
                {
                    match rx.recv().await
                    {
                        Some(DbMaintenanceMsg::Shutdown) => {
                            debug!("Maintenance worker is shutting down");
                            break;
                        },
                        Some(DbMaintenanceMsg::SaveMetadata(meta)) => {
                            debug!("Maintenance worker saves metadata to file");
                            match meta.save_to_file().await
                            {
                                Ok(_) => {},
                                Err(e) => {
                                    tx_to_db.send(
                                        create_internal_db_err_cmd(
                                            maintenance_worker_id, 
                                            &format!("Error: {}", e), 
                                            "Maintenance worker while Saving Metadata got error"
                                        )
                                    ).unwrap();
                                    // if send fails, this means db channel was
                                    // closed and graceful shutdown was not executed, thus we should also panic
                                }
                            }
                        },
                        Some(DbMaintenanceMsg::DeleteTable(table_meta)
                        ) => {
                            debug!("Maintenance worker deletes whole table files");
                            match delete_dir_with_contents(
                                &create_dir_path(
                                    &db_data_dir_path, 
                                    &table_meta.table_id(), 
                                    table_meta.table_name())
                            ).await
                            {
                                Ok(_) => {},
                                Err(e) => {
                                    tx_to_db.send(
                                        create_internal_db_err_cmd(
                                            maintenance_worker_id, 
                                            &format!("Error: {}", e), 
                                            "Maintenance worker while DELETING table data got error"
                                        )
                                    ).unwrap();
                                }
                            }
                            // TODO: instead of unwrapping we should here send a message to db about internal db error
                        },
                        None => {
                            debug!("Maintenance worker got None, other side of channel was closed, this happened before shutdown, DB CORRUPTED STATE");
                            break;
                        }
                    }
                }
            }
        );                    
        MaintenanceWorker::new(tx, saver_task_handle)
    }

    pub async fn await_task(self) -> Result<(), JoinError>
    {
        self.handle.await
    }

}

async fn delete_dir_with_contents(dir_path: &str) -> std::io::Result<()> 
{
    let dir_path = Path::new(dir_path);

    // Only folder with a parent can be deleted
    if let Some(parent_dir) = dir_path.parent() 
    {
        // We want to ensure that we will delete only folders that have 
        // their parent equal to DB_DATA_DIR
        if parent_dir == Path::new(DB_DATA_DIR)
        {
            // Try to remove, but ignore "NotFound" errors
            match tokio::fs::remove_dir_all(dir_path).await {
                Ok(_) => {
                    debug!("Successfully deleted directory: {:?}", dir_path);
                    return Ok(());
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // Directory doesn't exist - this is fine for deletion
                    debug!("Directory doesn't exist: '{:?}', no need to delete it, ignoring", dir_path);
                    return Ok(());
                }
                Err(e) => {
                    // Other errors (permission denied, etc.)
                    return Err(e);
                }
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput, 
        format!("Provided dir path: '{:?}' does not have parent equal to: '{}'", dir_path, DB_DATA_DIR)
    ))
}

fn create_internal_db_err_cmd(
    maintenance_worker_id: usize, 
    err: &str, 
    ctx: &str
) -> DbCmd
{
    return DbCmd::DbWorker(
        DbWorkerMsg::InternalError(
            maintenance_worker_id,
            QueryFailureMsg::new(
                MAINTENANCE_WORKER_QUERY_ID,
                MAINTENANCE_SAVE_META_TABLE_ID, 
                MultipleProblemsError::new_with_one_problem(err, ctx)
            )
        )
    );
}

fn create_dir_path(
    db_data_dir_path: &str,
    table_id: &Uuid, 
    table_name: &str, 
) -> String
{
    format!("{db_data_dir_path}/{table_id}_{table_name}")
}