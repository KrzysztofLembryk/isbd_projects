use crate::db::db_client::DbClient;
use crate::db::storage::metadata::TableId;
use crate::schemas::table::{TableSchema};
use crate::schemas::error::{Error};
use crate::db::manager::messages::{DbClientMsg, ResMsg};
use crate::routes::execute_db_cmd::execute_db_command;

use actix_web::{HttpResponse, Responder, delete, get, put, web};
use uuid::Uuid;
use validator::Validate;
use log::{info};

// TODO: remove code duplication by introducing helper functions/macro

#[get("/tables")]
async fn get_tables(
    db_client: web::Data<tokio::sync::RwLock<DbClient>>
) -> impl Responder
{
    info!("Fetching ALL TABLES DETAILS");

    let conn_id = Uuid::new_v4();
    let mut rx_conn = execute_db_command(
        &db_client, 
        &conn_id,
        DbClientMsg::GetTables(conn_id)
    ).await;

    let res = rx_conn.recv().await;

    return match res
    {
        Some(msg) =>{
            match msg
            {
                ResMsg::ResTables(tables) => 
                    HttpResponse::Ok()
                        .json(tables),
                _ => HttpResponse::InternalServerError()
                        .json("get_tables: got Wrong message from db"),
            }
        },
        None => HttpResponse::InternalServerError().json("get_tables: db closed its side of channel, couldnt receive data from db")
    };
}

#[get("/table/{table_id}")]
async fn get_table_details(
    table_id: web::Path<Uuid>, 
    db_client: web::Data<tokio::sync::RwLock<DbClient>>
) -> impl Responder
{
    info!("Fetching DETAILS for table with id: {}", table_id);

    let conn_id = Uuid::new_v4();
    let mut rx_conn = execute_db_command(
        &db_client, 
        &conn_id,
        DbClientMsg::GetTableDetails(conn_id, *table_id)
    ).await;

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
                        .json("get_tables: got Wrong message from db"),
            }
        },
        None => HttpResponse::InternalServerError().json("get_tables: db closed its side of channel, couldnt receive data from db")
    };
}

#[delete("/table/{table_id}")]
async fn delete_table(
    table_id: web::Path<TableId>,
    db_client: web::Data<tokio::sync::RwLock<DbClient>>,
) -> impl Responder
{
    info!("Deleting table with id: {}", table_id);

    let conn_id = Uuid::new_v4();
    let mut rx_conn = execute_db_command(
        &db_client, 
        &conn_id,
        DbClientMsg::DeleteTable(conn_id, *table_id)
    ).await;
    let res = rx_conn.recv().await;

    return match res
    {
        Some(msg) =>{
            match msg
            {
                ResMsg::ResDeleteTable(Ok(_)) => 
                    HttpResponse::Ok().json("Table has been deleted successfully"),
                ResMsg::ResDeleteTable(Err(e)) => 
                    HttpResponse::NotFound()
                        .json(Error::new(&format!("{}", e))),
                _ => HttpResponse::InternalServerError()
                        .json("delete_table: got Wrong message from db"),
            }
        },
        None => HttpResponse::InternalServerError().json("delete_table: db closed its side of channel, couldnt receive data from db")
    };
}

#[put("/table")]
async fn put_table(
    table_schema: web::Json<TableSchema>,
    db_client: web::Data<tokio::sync::RwLock<DbClient>>,
) -> impl Responder
{
    info!("Putting new table: '{}'", table_schema.name());
    if let Err(validation_err) = table_schema.validate()
    {
        return HttpResponse::BadRequest().json(Error::new(&format!("Table schema didnt pass validation: {}", validation_err)));
    }

    let conn_id = Uuid::new_v4();
    let mut rx_conn = execute_db_command(
        &db_client, 
        &conn_id,
        DbClientMsg::PutTable(conn_id, table_schema.into_inner())
    ).await;
    let res = rx_conn.recv().await;

    return match res
    {
        Some(msg) =>{
            match msg
            {
                ResMsg::ResPutTable(Ok(id)) => 
                    HttpResponse::Ok()
                        .json(id.to_string()),
                ResMsg::ResPutTable(Err(e)) => 
                    HttpResponse::BadRequest()
                        .json(Error::new(&format!("{}", e))),
                _ => HttpResponse::InternalServerError()
                        .json(Error::new("get_tables: got Wrong message from db")),
            }
        },
        None => HttpResponse::InternalServerError().json("get_tables: db closed its side of channel, couldnt receive data from db")
    };
}
