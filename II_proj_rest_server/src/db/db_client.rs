use crate::db::manager::messages::{DbClientMsg, ResMsg, DbCmd};
use uuid::Uuid;
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};
use tokio::sync::mpsc::error::SendError;

pub struct DbClient
{
    thread_id: Uuid,
    tx_db: UnboundedSender<DbCmd>,
    rx_server: UnboundedReceiver<ResMsg>
}

impl DbClient
{
    pub fn new(
        thread_id: Uuid,
        db_tx: UnboundedSender<DbCmd>,
        rx_server: UnboundedReceiver<ResMsg>
    ) -> DbClient
    {
        DbClient { thread_id, tx_db: db_tx, rx_server }
    }

    pub fn send_msg(&self, msg: DbClientMsg) -> Result<(), SendError<DbCmd>>
    {
        self.tx_db.send(DbCmd::Client(msg))
    }

    pub async fn recv_msg(&mut self) -> Result<ResMsg, String>
    {
        match self.rx_server.recv().await
        {
            Some(msg) => return Ok(msg),
            None => return Err(format!("DbClient::recv_msg:: channel closed for thread id:{}", self.thread_id))
        }
    }

    pub fn id(&self) -> Uuid
    {
        self.thread_id.clone()
    }
}