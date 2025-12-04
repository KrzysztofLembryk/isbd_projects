
use crate::db::manager::messages::{DbClientMsg, ResMsg};
use crate::db::db_client::DbClient;
use crate::routes::execute_db_cmd::execute_db_command;
use crate::schemas::query::{ExecuteQueryRequest};
use crate::schemas::error::{Error};
use actix_web::{HttpResponse, Responder, get, post, web};
use uuid::Uuid;

// TODO: remove code duplication by introducing helper functions/macro
#[get("/queries")]
async fn get_queries(
    db_client: web::Data<tokio::sync::RwLock<DbClient>>
) -> impl Responder
{
    println!("Fetching all QUERIES");

    let conn_id = Uuid::new_v4();
    let mut rx_conn = execute_db_command(
        &db_client, 
        &conn_id,
        DbClientMsg::GetQueries(conn_id)
    ).await;

    // Simulate long await for data
    let res = rx_conn.recv().await;

    return match res
    {
        Some(msg) =>{
            match msg
            {
                ResMsg::ResQueries(Ok(queries)) => 
                    HttpResponse::Ok()
                        .json(queries),
                ResMsg::ResQueries(Err(e)) => 
                    HttpResponse::NotFound()
                        .json(Error::new(&format!("{}", e))),
                _ => HttpResponse::InternalServerError()
                        .body("get_tables: got Wrong message from db"),
            }
        },
        None => HttpResponse::InternalServerError().body("get_tables: db closed its side of channel, couldnt receive data from db")
    };
}

#[get("/query/{query_id}")]
async fn get_query_info(
    query_id: web::Path<Uuid>,
    db_client: web::Data<tokio::sync::RwLock<DbClient>>
) -> impl Responder
{
    println!("Fetching query with id: {}", query_id);

    let conn_id = Uuid::new_v4();
    let mut rx_conn = execute_db_command(
        &db_client, 
        &conn_id,
        DbClientMsg::GetQueryDetails(conn_id, *query_id)
    ).await;

    // Simulate long await for data
    let res = rx_conn.recv().await;

    return match res
    {
        Some(msg) =>{
            match msg
            {
                ResMsg::ResQueryDetails(Ok(queries)) => 
                    HttpResponse::Ok()
                        .json(queries),
                ResMsg::ResQueryDetails(Err(e)) => 
                    HttpResponse::NotFound()
                        .json(Error::new(&format!("{}", e))),
                _ => HttpResponse::InternalServerError()
                        .body("get_tables: got Wrong message from db"),
            }
        },
        None => HttpResponse::InternalServerError().body("get_tables: db closed its side of channel, couldnt receive data from db")
    };
}

#[post("/query")]
async fn post_query(
    query: web::Json<ExecuteQueryRequest>,
    db_client: web::Data<tokio::sync::RwLock<DbClient>>
) -> impl Responder
{
    let conn_id = Uuid::new_v4();

    let mut rx_conn = execute_db_command(
        &db_client, 
        &conn_id,
        DbClientMsg::PostQuery(conn_id, query.query_definition())
    ).await;

    let res = rx_conn.recv().await;

    return match res
    {
        Some(msg) =>{
            match msg
            {
                ResMsg::ResPostQuery(Ok(id)) => 
                    HttpResponse::Ok()
                        .json(id),
                ResMsg::ResPostQuery(Err(e)) => 
                    HttpResponse::NotFound()
                        .json(Error::new(&format!("{}", e))),
                _ => HttpResponse::InternalServerError()
                        .body("get_tables: got Wrong message from db"),
            }
        },
        None => HttpResponse::InternalServerError().body("get_tables: db closed its side of channel, couldnt receive data from db")
    };
}

