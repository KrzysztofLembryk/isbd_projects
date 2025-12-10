use crate::db::manager::messages::{DbClientMsg, ResMsg};
use crate::db::db_client::DbClient;
use crate::routes::execute_db_cmd::execute_db_command;
use crate::schemas::query::{ExecuteQueryRequest};
use crate::schemas::error::{Error, MultipleProblemsError};
use actix_web::{HttpResponse, Responder, get, post, web};
use uuid::Uuid;
use log::{info};

// TODO: remove code duplication by introducing helper functions/macro
#[get("/queries")]
async fn get_queries(
    db_client: web::Data<tokio::sync::RwLock<DbClient>>
) -> impl Responder
{
    info!("Fetching all QUERIES");

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
                _ => HttpResponse::InternalServerError()
                        .json(Error::new("get_queries: INTERNAL DB ERROR: got Wrong message from db")),
            }
        },
        None => HttpResponse::InternalServerError().json(Error::new("get_queries: INTERNAL DB ERROR : db closed its side of channel, couldnt receive data from db"))
    };
}

#[get("/query/{query_id}")]
async fn get_query_info(
    query_id: web::Path<Uuid>,
    db_client: web::Data<tokio::sync::RwLock<DbClient>>
) -> impl Responder
{
    info!("Fetching query with id: {}", query_id);

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
                        .json(Error::new("get_query_indo: INTERNAL DB ERROR : got Wrong message from db")),
            }
        },
        None => HttpResponse::InternalServerError().json(Error::new("get_query_indo: INTERNAL DB ERROR : db closed its side of channel, couldnt receive data from db"))
    };
}

#[post("/query")]
async fn post_query(
    query: web::Json<ExecuteQueryRequest>,
    db_client: web::Data<tokio::sync::RwLock<DbClient>>
) -> impl Responder
{
    info!("POST query");
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
                    HttpResponse::BadRequest()
                        .json(MultipleProblemsError::new_with_one_problem(
                            &format!("{}", e),
                            "post_query context"
                        )),
                _ => HttpResponse::InternalServerError()
                        .json(MultipleProblemsError::new_with_one_problem(
                            "post_query: got Wrong message from db",
                            "INTERNAL DB ERROR"
                        )),
            }
        },
        None => HttpResponse::InternalServerError().json(MultipleProblemsError::new_with_one_problem(
            "post_query: db closed its side of channel, couldnt receive data from db",
            "INTERNAL DB ERROR"
        ))
    };
}

