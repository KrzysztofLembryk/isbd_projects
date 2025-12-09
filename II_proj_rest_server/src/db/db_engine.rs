use crate::db::manager::db_manager::{DbManager};
use crate::db::manager::messages::{DbClientMsg, DbCmd, ResMsg, DbWorkerMsg};

use tokio::task::JoinHandle;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use uuid::Uuid;
use log::{info, warn, debug, error};

enum BreakMsg
{
    DoBreak,
    NoMsg
}

pub struct DbEngine
{
    db_handle: JoinHandle<()>,
    tx_db: UnboundedSender<DbCmd>,
}

impl DbEngine
{
    pub async fn start(
        db_data_dir_path: &str, 
        metadata_file_path: &str,
        nbr_of_db_workers: usize,
    ) -> DbEngine
    {
        info!("Starting db engine.\ndb_data_dir_path: '{}',\nmetadata_file_path: '{}'", db_data_dir_path, metadata_file_path);

        let (tx_db, mut rx_db) = unbounded_channel::<DbCmd>();

        let mut db_manager = 
            DbManager::new(
                tx_db.clone(), 
                db_data_dir_path, 
                metadata_file_path,
                nbr_of_db_workers
                )
            .await
            .unwrap();

        let db_task_handle = tokio::spawn(async move {
                debug!("SPAWNING DB TASK");
                loop  
                {
                    match rx_db.recv().await
                    {
                        Some(DbCmd::Shutdown) => {
                            debug!("\n##########\nDB GOT SHUTDOWN\n##########");

                            db_manager.shutdown().await.unwrap();

                            break;
                        },
                        Some(DbCmd::Client(msg)) => {
                            match handle_client_cmd(msg, &mut db_manager)
                            {
                                BreakMsg::DoBreak => {
                                    db_manager.shutdown().await.unwrap();
                                    break;
                                },
                                _ => ()
                            }
                        },
                        Some(DbCmd::DbWorker(msg)) => {
                            match handle_worker_cmd(msg, &mut db_manager)
                            {
                                BreakMsg::DoBreak => {
                                    db_manager.shutdown().await.unwrap();
                                    break;
                                },
                                _ => ()
                            }
                        },
                        None => {
                            error!("Db task - all tx channels were closed, db rx channel returnen None");
                            db_manager.shutdown().await.unwrap();
                            break;
                        }
                    }
                }
            }
        );                    

        DbEngine {db_handle: db_task_handle, tx_db}
    }

    pub fn get_db_tx(&self) -> UnboundedSender<DbCmd>
    {
        self.tx_db.clone()
    }

    /// Method **consumes engine!!**
    pub async fn shutdown(self)
    {
        info!("Engine shutdown");
        match self.tx_db.send(DbCmd::Shutdown)
        {
            Err(e) => {
                error!("DbEngine::shutdown - db task closed its channel, db is already shutdown, ERROR: {}", e);
            },
            _ => ()
        }
        self.db_handle.await.unwrap();
    }
}

fn send_result_to_client(
    res_msg: ResMsg, 
    conn_id: &Uuid, 
    db_manager: &mut DbManager
)
{
    // TODO: Add erorr handling here
    db_manager.send_result(
        conn_id, 
        res_msg
    ).unwrap();
    db_manager.unregister_conn(conn_id);
}

fn handle_worker_cmd(
    worker_msg: DbWorkerMsg,
    db_manager: &mut DbManager,
) -> BreakMsg
{
    match worker_msg
    {
        DbWorkerMsg::QueryCompleted(worker_id, success_msg) => {
            match db_manager.handle_completed_query(worker_id, success_msg)
            {
                Err(e) => {
                    error!("DbTask::handle_worker_cmd: got error when handling QueryCompleted: {}", e);
                    return BreakMsg::DoBreak;
                },
                _ => {}
            }
        },
        DbWorkerMsg::QueryFailed(worker_id, fail_msg) => {
            match db_manager.handle_failed_query(worker_id, fail_msg)
            {
                Err(e) => {
                    error!("DbTask::handle_worker_cmd got error when handling QueryFailed: {}", e);
                    return BreakMsg::DoBreak;
                },
                _ => {}
            }
        },
        DbWorkerMsg::InternalError(worker_id, error_msg) => {
            error!("DbTask::handle_worker_cmd: from worker: '{}' got INTERNAL ERROR: {:?}", worker_id, error_msg);
            return BreakMsg::DoBreak;
        },
        _ => {
            error!("DbTask::handleWorker_cmd DbManager got unsupported msg from DbWorker, DB is in CORRUPTED STATE, initiating shutdown");
            return  BreakMsg::DoBreak;
        }    
    }
    return BreakMsg::NoMsg;
}

fn handle_client_cmd(
    client_msg: DbClientMsg, 
    db_manager: &mut DbManager
) -> BreakMsg
{
    // TODO: probably this can be DONE BETTER, less code repetition
    match client_msg
    {
        DbClientMsg::Register(id, tx) => {
            db_manager.register_conn(&id, tx);
        },
        DbClientMsg::GetTables(conn_id) => {
            let res = db_manager.get_tables();

            send_result_to_client(
                ResMsg::ResTables(res), 
                &conn_id, 
                db_manager
            );
        },
        DbClientMsg::GetTableDetails(conn_id, table_id) => {
            let res = db_manager.get_table_details(&table_id);

            send_result_to_client(
                ResMsg::ResTableDetails(res), 
                &conn_id, 
                db_manager
            );
        },
        DbClientMsg::DeleteTable(conn_id, table_id) => {
            let res = db_manager.delete_table(&table_id);
            send_result_to_client(
                ResMsg::ResDeleteTable(res), 
                &conn_id, 
                db_manager
            );
        },
        DbClientMsg::PutTable(conn_id, table_schema) => {
            let res = db_manager.put_table(&table_schema);

            send_result_to_client(
                ResMsg::ResPutTable(res),
                &conn_id, 
                db_manager
            );
        },
        DbClientMsg::GetQueries(conn_id) => {
            let res = db_manager.get_queries();

            send_result_to_client(
                ResMsg::ResQueries(res),
                &conn_id, 
                db_manager
            );
        },
        DbClientMsg::GetQueryDetails(conn_id, query_id) => {
            let res = db_manager
                .get_query_details(&query_id);

            send_result_to_client(
                ResMsg::ResQueryDetails(res),
                &conn_id, 
                db_manager
            );
        },
        DbClientMsg::PostQuery(conn_id, q) => {
            let res = db_manager
                .post_query(q);

            send_result_to_client(
                ResMsg::ResPostQuery(res),
                &conn_id, 
                db_manager
            );
        },
        DbClientMsg::GetQueryRes(conn_id, (query_id, row_count, do_flush)) => {
            let res = db_manager.get_query_result(
                &query_id, 
                row_count, 
                do_flush
            ); 

            send_result_to_client(
                ResMsg::ResQuery(res),
                &conn_id, 
                db_manager
            );
        },
        DbClientMsg::GetFailedQuery(conn_id, query_id) => {
            let res = db_manager.get_query_failed(
                &query_id, 
            ); 

            send_result_to_client(
                ResMsg::ResFailedQuery(res),
                &conn_id, 
                db_manager
            );
        },
    }
    return BreakMsg::NoMsg;
}