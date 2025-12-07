use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use crate::db::errors::DbError;
use crate::db::manager::messages::{BaseQueryDataInfo, DbCmd, DbWorkerMsg, QueryCompletionMsg, QueryData, QueryFailureMsg, SelectQData}; 
use crate::db::storage::metadata::{TableMetadata};
use crate::schemas::query::QueryResult;
use crate::schemas::error::{Error, MultipleProblemsError, Problem};

use log::{info, warn, debug, error};

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
                DbWorkerMsg::ExecQuery(worker_id, q_data) => {
                    if worker_id == self.id
                    {
                        self.execute_query(q_data).await;
                    }
                    else // worker_id != self.id
                    {
                        self.send_msg_to_db(
                            DbWorkerMsg::InternalError(
                                worker_id, 
                                QueryFailureMsg::new(
                                    q_data.query_id(), 
                                    q_data.table_id(), 
                                    MultipleProblemsError::new_with_one_problem(
                                        &format!("QueryWorker::run: Worker: '{}' got message with wrong worker id: '{}'", self.id, worker_id)
                                        , 
                                        "In proper functioning db this should never happen, there is probably a bug somewhere"
                                    )
                        )));
                    }
                },
                DbWorkerMsg::Shutdown => {
                    debug!("Worker '{}' is shutting down", self.id);
                    break;
                },
                _ => {
                    warn!("Worker '{}' got unsupported msg, Doing nothig", self.id);
                }
            }
        }
    }

    async fn execute_query(&self, q_data: QueryData)
    {
        let worker_id = self.id;
        match q_data
        {
            QueryData::SelectQ(s_q) => {
                println!("Worker '{}' got SELECT QUERY: {:?}", worker_id, s_q);

                let res_msg = QueryWorker::handle_select(worker_id, s_q).await;
                self.send_msg_to_db(res_msg);
            },
            QueryData::CopyQ(c_q) => {
                println!("Worker '{}' got COPY QUERY: {:?}", worker_id, c_q);

                let table_meta = c_q.table_metadata();
                let failure_msg = QueryFailureMsg::new(
                    c_q.query_id(), 
                    table_meta.table_id(), 
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

    async fn handle_select(
        worker_id: usize,
        s_q: SelectQData
    ) -> DbWorkerMsg 
    {
        let table_meta: &TableMetadata = s_q.table_metadata();
        let query_id = s_q.query_id();
        let table_id = table_meta.table_id();
        let query_result = table_meta.read_table().await;

        match query_result
        {
            Ok((q_res, n_rows)) => {
                return 
                    DbWorkerMsg::QueryCompleted(
                        worker_id,
                        QueryCompletionMsg::new(
                            query_id, 
                            table_id, 
                            n_rows, 
                            Some(q_res)
                ));
            },
            Err(e) => {
                match e
                {
                    DbError::InternalDbError(e) => {
                        return DbWorkerMsg::InternalError(
                            worker_id,
                            QueryFailureMsg::new(
                                query_id, 
                                table_id, 
                                MultipleProblemsError::new_with_one_problem(
                                    &e,
                                    &format!("QueryWorker::handle_select:: When reading table: '{}' we got error", table_meta.table_name())
                            )
                        ));
                    },
                    // We treat IOErrors as internalDbErrors and want to 
                    // shutdown db, since here we are handling SELECT so only 
                    // reading data from db, so this means that DbMetadata has
                    // info about given table, but we couldnt read it (i.e. 
                    // somebody removed files, or corrupted them).
                    // Thus our whole db is in corrupted state and we want to 
                    // end it's execution
                    DbError::IoError(e) => {
                        return DbWorkerMsg::InternalError(
                            worker_id,
                            QueryFailureMsg::new(
                                query_id, 
                                table_id, 
                                MultipleProblemsError::new_with_one_problem(
                                    &e.to_string(),
                                    &format!("QueryWorker::handle_select:: When reading table: '{}' we got IO error", table_meta.table_name())
                            )
                        ));
                    },
                    _ => {
                        return DbWorkerMsg::QueryFailed(
                            worker_id,
                            QueryFailureMsg::new(
                            query_id, 
                            table_id, 
                            MultipleProblemsError::new_with_one_problem(
                                &e.to_string(),
                                &format!("QueryWorker::handle_select:: When reading table: '{}' we got error", table_meta.table_name())
                            )
                        ));
                    }
                }
            }
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
