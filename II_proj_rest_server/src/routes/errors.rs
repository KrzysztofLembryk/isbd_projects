use crate::db::manager::messages::{DbClientMsg, ResMsg};
use crate::db::db_client::DbClient;
use crate::routes::execute_db_cmd::execute_db_command;
use crate::schemas::error::{Error};
use crate::db::errors::DbError;
use actix_web::{HttpResponse, Responder, get, web};
use uuid::Uuid;
use log::{info};

#[get("/error/{query_id}")]
async fn get_failed_query(
    query_id: web::Path<Uuid>,
    db_client: web::Data<tokio::sync::RwLock<DbClient>>
) -> impl Responder
{
    info!("Fetching failed query: '{}'", *query_id);

    let conn_id = Uuid::new_v4();
    let mut rx_conn = execute_db_command(
        &db_client, 
        &conn_id,
        DbClientMsg::GetFailedQuery(
            conn_id, 
            *query_id, 
        )
    ).await;

    // Simulate long await for data
    let res = rx_conn.recv().await;

    return match res
    {
        Some(msg) =>{
            match msg
            {
                ResMsg::ResFailedQuery(Ok(queries)) => 
                    HttpResponse::Ok()
                        .json(queries),
                ResMsg::ResFailedQuery(Err(e)) => 
                {
                    match e
                    {
                        DbError::NotFound(e) => {
                            HttpResponse::NotFound()
                                .json(Error::new(&format!("{}", e)))
                        },
                        DbError::IoError(e) => {
                            HttpResponse::InternalServerError()
                                .json(Error::new(&format!("{}", e)))
                        },
                        DbError::InternalDbError(e) => {
                            HttpResponse::InternalServerError()
                                .json(Error::new(&format!("{}", e)))
                        },
                        other_err => {
                            HttpResponse::BadRequest()
                                .json(Error::new(&format!("{}", other_err)))
                        }
                    }
                }
                _ => HttpResponse::InternalServerError()
                        .json("get_failed_query: got Wrong message from db"),
            }
        },
        None => HttpResponse::InternalServerError().json("get_tables: db closed its side of channel, couldnt receive data from db")
    };
}