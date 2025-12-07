use II_proj_rest_server::routes::tables::{get_tables, get_table_details, put_table, delete_table};
use II_proj_rest_server::routes::queries::{get_queries, get_query_info, post_query};
use II_proj_rest_server::db::constants::{DB_DATA_DIR, METADATA_FILE_PATH, MAX_DB_WORKERS};
use actix_web::{App, HttpServer, web};
use II_proj_rest_server::db::manager::messages::{DbClientMsg, DbCmd, ResMsg};
use II_proj_rest_server::db::db_client::DbClient;
use II_proj_rest_server::db::db_engine::DbEngine;

use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use uuid::Uuid;
use env_logger::Builder;
use log::LevelFilter;


#[actix_web::main]
async fn main() -> std::io::Result<()>
{
    // Logging
    Builder::new()
        .filter_level(LevelFilter::Debug)
        .format_timestamp_millis()
        .init();

    // STREAMING large amounts of data Actix:
    // https://github.com/actix/actix-web/issues/1653

    // Request body from HTTP request, and getting data to buffer with given 
    // SIZE: https://actix.rs/docs/request/

    let db_engine = DbEngine::start(
        DB_DATA_DIR, 
        METADATA_FILE_PATH,
        MAX_DB_WORKERS
    ).await;
    let tx_db = db_engine.get_db_tx();

    HttpServer::new(move || {
        let db_client = create_db_client(tx_db.clone());
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

    db_engine.shutdown().await;

    Ok(())
}

fn create_db_client(tx_db: UnboundedSender<DbCmd>) -> DbClient
{
    let id = Uuid::new_v4();
    let (tx_thread, rx_thread) = unbounded_channel::<ResMsg>();

    tx_db.send(DbCmd::Client(
                DbClientMsg::Register(id.clone(), tx_thread)
            )
        ).unwrap();

    DbClient::new(id, tx_db, rx_thread)
}
