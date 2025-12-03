use crate::db::constants::DB_DATA_DIR;
use crate::db::manager::messages::{DbMaintenanceMsg};
use crate::db::errors::DbError;

use std::fmt::format;
use std::path::Path;
use uuid::Uuid;
use tokio::{sync::mpsc::{UnboundedSender, unbounded_channel}, task::JoinError};
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
    pub fn spawn(db_data_dir_path: &str) -> MaintenanceWorker
    {
        let (tx, mut rx) = unbounded_channel::<DbMaintenanceMsg>();
        let path = String::from(db_data_dir_path);

        let saver_task_handle = tokio::spawn(async move {
                let db_data_dir_path = path.clone();
                loop  
                {
                    // TODO: Add error handling here, we need tx for main db task so that we can inform it if critical error happened
                    match rx.recv().await
                    {
                        Some(DbMaintenanceMsg::Shutdown) => {break;},
                        Some(DbMaintenanceMsg::SaveMetadata(meta)) => {
                            meta.save_to_file().await.unwrap();
                        },
                        Some(DbMaintenanceMsg::DeleteTable(table_meta)
                        ) => {
                            // TODO: add error handling
                            delete_dir_with_contents(
                                &create_dir_path(
                                    &db_data_dir_path, 
                                    table_meta.table_id(), 
                                    table_meta.table_name())
                            ).await.unwrap();
                        },
                        None => {break;}
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
            tokio::fs::remove_dir_all(dir_path).await?;
            return Ok(());
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput, 
        format!("Provided dir path: '{:?}' does not have parent equal to: '{}'", dir_path, DB_DATA_DIR)
    ))
}

fn create_dir_path(
    db_data_dir_path: &str,
    table_id: &Uuid, 
    table_name: &str, 
) -> String
{
    format!("{db_data_dir_path}/{table_id}_{table_name}")
}