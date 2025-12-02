use crate::db::manager::messages::{MetaSaverMessage};
use tokio::{sync::mpsc::{UnboundedSender, unbounded_channel}, task::JoinError};
use crate::db::errors::DbError;

pub struct MetadataSaver
{
    tx_saver: UnboundedSender<MetaSaverMessage>,
    handle: tokio::task::JoinHandle<()>,
}

impl MetadataSaver
{
    pub fn new(
        tx_saver: UnboundedSender<MetaSaverMessage>,
        handle: tokio::task::JoinHandle<()>
    ) -> MetadataSaver
    {
        MetadataSaver {tx_saver, handle}
    }

    pub fn send_msg(&self, msg: MetaSaverMessage) -> Result<(), DbError>
    {
        self.tx_saver
            .send(msg)
            .map_err(|e| DbError::InternalDbError(
                format!("DbManager::put_table: failed to send save message: {}", e)
            ))
    }
    /// Function spawns new task that saves, received by msg, metadata to file
    /// It returns MetadataSaver that stores task handle and channel transmitter
    pub fn spawn() -> MetadataSaver
    {
        let (tx_meta_saver, mut rx_meta_saver) = unbounded_channel::<MetaSaverMessage>();

        let saver_task_handle = tokio::spawn(async move {
                loop  
                {
                    match rx_meta_saver.recv().await
                    {
                        Some(MetaSaverMessage::Shutdown) => {break;},
                        Some(MetaSaverMessage::SaveMetadata(meta)) => {
                            meta.save_to_file().await.unwrap();
                        },
                        None => {break;}
                    }
                }
            }
        );                    

        MetadataSaver::new(tx_meta_saver, saver_task_handle)
    }

    pub async fn await_task(self) -> Result<(), JoinError>
    {
        self.handle.await
    }
}