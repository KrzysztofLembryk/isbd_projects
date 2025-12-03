use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::db::manager::messages::{DbCmd, DbWorkerMsg};

pub struct QueryWorker
{
    id: usize,
    tx: UnboundedSender<DbCmd>,
    rx: UnboundedReceiver<DbWorkerMsg>
}

impl QueryWorker
{
    pub fn new(
        id: usize, 
        tx: UnboundedSender<DbCmd>, 
        rx: UnboundedReceiver<DbWorkerMsg>
    ) -> QueryWorker
    {
        QueryWorker { id, tx, rx }
    }

    pub async fn run(&mut self)
    {

    }
}