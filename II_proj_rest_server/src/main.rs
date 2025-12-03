use II_proj_rest_server::routes::tables::{get_tables, get_table_details, put_table, delete_table};
use II_proj_rest_server::routes::queries::{get_queries, get_query_info, post_query};
use II_proj_rest_server::db::constants::{DB_DATA_DIR, METADATA_FILE_PATH};
use actix_web::{App, HttpServer, web};
use II_proj_rest_server::db::manager::db_manager::{DbManager};
use II_proj_rest_server::db::manager::messages::{DbClientMsg, DbCmd, ResMsg};
use II_proj_rest_server::db::db_client::DbClient;

use tokio::sync::mpsc::{unbounded_channel};
use uuid::Uuid;

enum BreakMsg
{
    DoBreak,
    NoMsg
}

#[actix_web::main]
async fn main() -> std::io::Result<()>
{
    // STREAMING large amounts of data Actix:
    // https://github.com/actix/actix-web/issues/1653

    // Request body from HTTP request, and getting data to buffer with given SIZE
    // https://actix.rs/docs/request/

    // NEW IDEA:
    // 1. Before running HttpServer we spawn thread/tokio_task or sth with 
    // db_manager
    // 2. We create channels through which HttpServer tasks will register in our
    // db_manager executor, and send to it its channels so that db_manager can
    // send and receive data from server endpoints
    // 3. Inside db_manager we will spawn new tasks for given Server Request
    // and pass clones of given tables metadata, if these tasks will read data 
    // we can give them clone of channel to send data directly to the server Thread

    // TODO: db_manager should be inside ENGINE struct, and here we should only
    // run engine.start()
    let mut db_manager = 
        DbManager::new(DB_DATA_DIR, METADATA_FILE_PATH).await.unwrap();

    let (tx_db, mut rx_db) = unbounded_channel::<DbCmd>();
    let tx_db_clone = tx_db.clone();

    db_manager.init_worker_manager(tx_db.clone());

    let db_task = tokio::spawn(async move {
            println!("SPAWNING DB TASK");
            loop  
            {
                // TODO: probably this can be DONE BETTER, less code repetition
                match rx_db.recv().await
                {
                    Some(DbCmd::Shutdown) => {
                        println!("\n##########\nDB GOT SHUTDOWN\n##########");

                        db_manager.shutdown().await.unwrap();

                        break;
                    },
                    Some(DbCmd::Client(msg)) => {
                        match handle_client_cmd(msg, &mut db_manager).await
                        {
                            BreakMsg::DoBreak => {
                                break;
                            },
                            _ => ()
                        }
                    },
                    Some(DbCmd::DbWorker(msg)) => {

                    },
                    None => {
                        println!("Db task - rx_dv.recv channel was closed");
                        // TODO: shutdown of db, db workers etc
                        break;
                    }
                }
            }
        }
    );                    

    HttpServer::new(move || {
        // Here we will do thread registration, we will get DB tx and we will send our tx to db
        let id = Uuid::new_v4();
        let db_tx = tx_db.clone();
        let (tx_thread, rx_thread) = unbounded_channel::<ResMsg>();
        db_tx.send(DbCmd::Client(
                    DbClientMsg::Register(id.clone(), tx_thread)
                )
            ).unwrap();

        let db_client = DbClient::new(id, db_tx, rx_thread);

        App::new()
            // db_client will be local for each thread
            .app_data(web::Data::new(tokio::sync::RwLock::new(db_client)))
            .service(get_tables)
            .service(get_table_details)
            .service(delete_table)
            .service(put_table)
            .service(get_queries)
            .service(get_query_info)
            .service(post_query)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;


    tx_db_clone.send(DbCmd::Shutdown).unwrap();
    db_task.await.unwrap();

    Ok(())
}

fn send_result(res_msg: ResMsg, conn_id: &Uuid, db_manager: &mut DbManager)
{
    // TODO: Add erorr handling here
    db_manager.send_result(
        conn_id, 
        res_msg
    ).unwrap();
    db_manager.unregister(conn_id);
}

async fn handle_client_cmd(
    client_msg: DbClientMsg, 
    db_manager: &mut DbManager
) -> BreakMsg
{
    match client_msg
    {
        DbClientMsg::Register(id, tx) => {
            db_manager.register(&id, tx);
        }
        DbClientMsg::GetTables(conn_id) => {
            let res = db_manager.get_tables();

            send_result(
                ResMsg::ResTables(res), 
                &conn_id, 
                db_manager
            );
        }
        DbClientMsg::GetTableDetails(conn_id, id) => {
            let res = db_manager.get_table_details(&id);

            send_result(
                ResMsg::ResTableDetails(res), 
                &conn_id, 
                db_manager
            );
        },
        DbClientMsg::DeleteTable(conn_id, table_id) => {
            let res = db_manager.mark_table_to_delete(&table_id);

            match db_manager.delete_table(&table_id)
            {
                Ok(t_meta) => {
                    // No queries running on table, so we can 
                    // schedule table deletion
                    let res = db_manager
                                .schedule_table_deletion(t_meta);
                    send_result(
                        ResMsg::ResDeleteTable(res), 
                        &conn_id, 
                        db_manager
                    );
                },
                Err(e) => {
                    // This means we cannot yet delete table, since
                    // there are some queries running on it
                    println!("DbTask::DeleteTable: {}", e);
                    send_result(
                        ResMsg::ResDeleteTable(res), 
                        &conn_id, 
                        db_manager
                    );
                }
            }
        },
        DbClientMsg::PutTable(conn_id, table_schema) => {
            let res = db_manager.put_table(&table_schema);

            send_result(
                ResMsg::ResPutTable(res),
                &conn_id, 
                db_manager
            );
        },
        DbClientMsg::GetQueries(conn_id) => {
            let res = db_manager.get_queries();

            send_result(
                ResMsg::ResQueries(res),
                &conn_id, 
                db_manager
            );
        },
        DbClientMsg::GetQueryDetails(conn_id, query_id) => {
            let res = db_manager
                .get_query_details(&query_id);

            send_result(
                ResMsg::ResQueryDetails(res),
                &conn_id, 
                db_manager
            );
        },
        DbClientMsg::PostQuery(conn_id, q) => {
            let res = db_manager
                .post_query(q);

            send_result(
                ResMsg::ResPostQuery(res),
                &conn_id, 
                db_manager
            );
        }
    }

    return BreakMsg::NoMsg;
}