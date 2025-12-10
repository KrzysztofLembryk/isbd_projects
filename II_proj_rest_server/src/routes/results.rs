use crate::db::errors::DbError;
use crate::db::manager::messages::{DbClientMsg, ResMsg};
use crate::db::db_client::DbClient;
use crate::routes::execute_db_cmd::execute_db_command;
use crate::schemas::error::{Error};
use actix_web::{HttpResponse, Responder, get, web};
use uuid::Uuid;
use serde::Deserialize;
use log::{info};

#[derive(Deserialize)]
struct QueryBody {
    #[serde(default, rename= "rowLimit")]
    row_limit: i32,
    #[serde(default, rename = "flushResult")]
    flush_result: bool,
}

#[get("/result/{query_id}")]
async fn get_query_result(
    query_id: web::Path<Uuid>,
    query_body: Option<web::Json<QueryBody>>,
    db_client: web::Data<tokio::sync::RwLock<DbClient>>
) -> impl Responder
{
    // default if no body provided
    let query_body = query_body.map(|b| b.into_inner()).unwrap_or(QueryBody {
        row_limit: 0,
        flush_result: false,
    });
    
    info!("Fetching result for query: '{}', with row limit: '{}' and flush set to: '{}' ", query_id, query_body.row_limit, query_body.flush_result);

    if query_body.row_limit < 0 
    {
        return HttpResponse::BadRequest()
                .json(Error::new(&format!("We do not allow row limit to be negative: {}", query_body.row_limit)))
    }

    let conn_id = Uuid::new_v4();
    let mut rx_conn = execute_db_command(
        &db_client, 
        &conn_id,
        DbClientMsg::GetQueryRes(
            conn_id, 
            (*query_id, query_body.row_limit as usize, query_body.flush_result)
        )
    ).await;

    let res = rx_conn.recv().await;

    return match res
    {
        Some(msg) =>{
            match msg
            {
                ResMsg::ResQuery(Ok(queries)) => 
                    HttpResponse::Ok()
                        .json(queries),
                ResMsg::ResQuery(Err(e)) => 
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
                        .json("get_query_result: got Wrong message from db"),
            }
        },
        None => HttpResponse::InternalServerError().json("get_query_result: db closed its side of channel, couldnt receive data from db")
    };
}