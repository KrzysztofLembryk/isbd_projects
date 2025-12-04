use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{db::manager::messages::{DbCmd, DbWorkerMsg, QueryComlpetionMsg, QueryData, QueryFailureMsg}, schemas::error::{Error, MultipleProblemsError, Problem}};


pub struct QueryWorker
{
    id: usize,
    tx_to_db: UnboundedSender<DbCmd>,
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
        QueryWorker { id, tx_to_db: tx, rx }
    }

    pub async fn run(&mut self)
    {
        while let Some(msg) = self.rx.recv().await
        {
            match msg
            {
                DbWorkerMsg::DoQuery(worker_id, q_data) => {
                    if worker_id != self.id
                    {
                        println!("Got wrong worker_id, this query should Fail");
                    }
                    else 
                    {
                        // Simulating working on query
                        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                        self.handle_do_query(q_data);
                    }
                },
                DbWorkerMsg::Shutdown => {
                    println!("Worker '{}' is shutting down", self.id);
                    break;
                },
                _ => {
                    println!("Worker '{}' got unsupported msg, Doing nothig", self.id);
                }
            }
        }
    }

    fn handle_do_query(&self, q_data: QueryData)
    {
        let worker_id = self.id;
        match q_data
        {
            QueryData::SelectQ(s_q) => {
                println!("Worker '{}' got SELECT QUERY: {:?}", worker_id, s_q);

                let table_meta = s_q.table_metadata();
                let completion_msg = QueryComlpetionMsg::new(
                    *s_q.query_id(), 
                    *table_meta.table_id(), 
                    None
                );

                self.send_msg_to_db(
                    DbWorkerMsg::QueryCompleted(
                        worker_id, 
                        completion_msg
                ));
            },
            QueryData::CopyQ(c_q) => {
                println!("Worker '{}' got COPY QUERY: {:?}", worker_id, c_q);

                let table_meta = c_q.table_metadata();
                let failure_msg = QueryFailureMsg::new(
                    *c_q.query_id(), 
                    *table_meta.table_id(), 
                    MultipleProblemsError::new(
                        vec![Problem::new(
                            &Error::new("Copy QUery failed - not impl"), 
                            "TEST CONTEXT")
                        ]
                    )
                );

                self.send_msg_to_db(DbWorkerMsg::QueryFailed(
                    worker_id, 
                    failure_msg
                ));
            },
        }
    }

    fn send_msg_to_db(&self, msg: DbWorkerMsg)
    {
        // If we cannot send msg to db it means channel was
        // closed so we should panic and end execution
        // We could use HealthChecks Algorithm from Distributed Systems
        self.tx_to_db.send(DbCmd::DbWorker(msg)).unwrap();
    }
}
