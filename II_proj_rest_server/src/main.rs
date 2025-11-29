use II_proj_rest_server::db::storage::metadata_structs::DbMetadata;
use II_proj_rest_server::routes::tables::{get_tables, get_table_details, put_table, delete_table};
use II_proj_rest_server::routes::queries::{get_queries, get_query_info, post_query};
use II_proj_rest_server::db::constants::{DB_DATA_DIR};
use actix_web::{App, HttpServer, web};
use II_proj_rest_server::db::db_manager::{DbManager, TaskMessage};

use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

#[actix_web::main]
async fn main() -> std::io::Result<()>
{
    // NEW IDEA:
    // 1. Before running HttpServer we spawn thread/tokio_task or sth with 
    // db_manager
    // 2. We create channels through which HttpServer tasks will register in our
    // db_manager executor, and send to it its channels so that db_manager can
    // send and receive data from server endpoints
    // 3. Inside db_manager we will spawn new tasks for given Server Request
    // and pass clones of given tables metadata, if these tasks will read data 
    // we can give them clone of channel to send data directly to the server Thread

    // OLD IDEA:
    // 1. We initialize db_manager
    // 2. We initialize our Database (i.e. DbMetadata inside db_manager)
    // 3. We create Arc<RwLock<db_manager>>
    //      - RwLock - read write lock, allows many readers, but one writer
    //      - Arc - since we want to be able to share db_manager between Actix 
    //              threads

    let (tx_msg, mut rx_msg) = unbounded_channel::<TaskMessage>();
    let mut db_manager = DbManager::new(DB_DATA_DIR, tx_msg.clone());

    db_manager.init_db().await.unwrap();

    let metadata_saver_task_handle = tokio::spawn(async move {
            loop  
            {
                match rx_msg.recv().await
                {
                    Some(TaskMessage::Shutdown) => {break;},
                    Some(TaskMessage::SaveMetadata(meta)) => {
                        DbMetadata::save_snapshot_to_file(meta).await.unwrap()
                    },
                    None => {break;}
                }
            }
        }
    );                    

    // Internally, web::Data uses Arc
    // --> So we have Arc<RwLock<DbManager>>
    let db_manager = web::Data::new(tokio::sync::RwLock::new(db_manager));
    let manager_clone = db_manager.clone();

    HttpServer::new(move || {
        App::new()
            .app_data(db_manager.clone())
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


    // After Httpserver ends its execution, we acquire lock and tell 
    // MetadataSaverTask to save metadata 
    let db_manager = manager_clone.read().await;

    db_manager.save_metadata().unwrap();
    db_manager.shutdown().unwrap();

    // Need to wait for MetadataSaverTask to end its execution
    metadata_saver_task_handle.await?;

    Ok(())
}
