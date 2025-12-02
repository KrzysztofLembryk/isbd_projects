use crate::db::db_client::DbClient;
use crate::db::storage::metadata::TableId;
use crate::schemas::table::{TableSchema};
use crate::schemas::error::{Error};
use crate::db::manager::messages::{DbCmd, ResMsg};
use crate::routes::execute_db_cmd::execute_db_command;

use actix_web::{HttpResponse, Responder, delete, get, put, web};
use uuid::Uuid;
use tokio::time::{sleep, Duration};

// TODO: remove code duplication by introducing helper functions/macro

#[get("/tables")]
async fn get_tables(
    db_client: web::Data<tokio::sync::RwLock<DbClient>>
) -> impl Responder
{
    println!("Fetching ALL TABLES DETAILS");

    let conn_id = Uuid::new_v4();
    let mut rx_conn = execute_db_command(
        &db_client, 
        &conn_id,
        DbCmd::GetTables(conn_id)
    ).await;

    // Simulate long await for data
    sleep(Duration::from_secs(10)).await;
    let res = rx_conn.recv().await;

    return match res
    {
        Some(msg) =>{
            match msg
            {
                ResMsg::ResTables(Ok(tables)) => 
                    HttpResponse::Ok()
                        .json(tables),
                ResMsg::ResTables(Err(e)) => 
                    HttpResponse::NotFound()
                        .json(Error::new(&format!("{}", e))),
                _ => HttpResponse::InternalServerError()
                        .body("get_tables: got Wrong message from db"),
            }
        },
        None => HttpResponse::InternalServerError().body("get_tables: db closed its side of channel, couldnt receive data from db")
    };
}

#[get("/table/{table_id}")]
async fn get_table_details(
    table_id: web::Path<Uuid>, 
    db_client: web::Data<tokio::sync::RwLock<DbClient>>
) -> impl Responder
{
    println!("Fetching DETAILS for table with id: {}", table_id);

    let conn_id = Uuid::new_v4();
    let mut rx_conn = execute_db_command(
        &db_client, 
        &conn_id,
        DbCmd::GetTableDetails(conn_id, *table_id)
    ).await;

    sleep(Duration::from_secs(10)).await;

    let res = rx_conn.recv().await;

    return match res
    {
        Some(msg) =>{
            match msg
            {
                ResMsg::ResTableDetails(Ok(tables)) => 
                    HttpResponse::Ok()
                        .json(tables),
                ResMsg::ResTableDetails(Err(e)) => 
                    HttpResponse::NotFound()
                        .json(Error::new(&format!("{}", e))),
                _ => HttpResponse::InternalServerError()
                        .body("get_tables: got Wrong message from db"),
            }
        },
        None => HttpResponse::InternalServerError().body("get_tables: db closed its side of channel, couldnt receive data from db")
    };
}

#[delete("/table/{table_id}")]
async fn delete_table(
    table_id: web::Path<TableId>,
    db_client: web::Data<tokio::sync::RwLock<DbClient>>,
) -> impl Responder
{
    // We will get write for db_manager, firstly we will set flag that this 
    // table is to be deleted, so that UPCOMING tables/queries requests will 
    // not be able to see and operate on this table
    // We will have hashmap - 
    //  {
    //      table_name/id: (delete_flag, n_queries_operating_on_table)
    //  }
    // For every query we will spawn task, that will communicate with db_manager
    // via channels, 
    // We will spawn another TASK before running server
    println!("Fetching table with id: {}", table_id);

    let conn_id = Uuid::new_v4();
    let mut rx_conn = execute_db_command(
        &db_client, 
        &conn_id,
        DbCmd::DeleteTable(conn_id, *table_id)
    ).await;
    let res = rx_conn.recv().await;

    return match res
    {
        Some(msg) =>{
            match msg
            {
                ResMsg::ResDeleteTable(Ok(_)) => 
                    HttpResponse::Ok().body("Table deleted succesfully"),
                ResMsg::ResDeleteTable(Err(e)) => 
                    HttpResponse::BadRequest()
                        .json(Error::new(&format!("{}", e))),
                _ => HttpResponse::InternalServerError()
                        .body("get_tables: got Wrong message from db"),
            }
        },
        None => HttpResponse::InternalServerError().body("get_tables: db closed its side of channel, couldnt receive data from db")
    };
}

#[put("/table")]
async fn put_table(
    table_schema: web::Json<TableSchema>,
    db_client: web::Data<tokio::sync::RwLock<DbClient>>,
) -> impl Responder
{
    println!("Putting new table");

    let conn_id = Uuid::new_v4();
    let mut rx_conn = execute_db_command(
        &db_client, 
        &conn_id,
        DbCmd::PutTable(conn_id, table_schema.into_inner())
    ).await;
    let res = rx_conn.recv().await;

    return match res
    {
        Some(msg) =>{
            match msg
            {
                ResMsg::ResPutTable(Ok(id)) => 
                    HttpResponse::Ok()
                        .body(id.to_string()),
                ResMsg::ResPutTable(Err(e)) => 
                    HttpResponse::BadRequest()
                        .json(Error::new(&format!("{}", e))),
                _ => HttpResponse::InternalServerError()
                        .body("get_tables: got Wrong message from db"),
            }
        },
        None => HttpResponse::InternalServerError().body("get_tables: db closed its side of channel, couldnt receive data from db")
    };
}
