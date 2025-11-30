use crate::schemas::query::{
    ShallowQuery, 
    QueryStatus, 
    Query, 
    AllowedQuery, 
    CopyQuery, 
    SelectQuery,
    ExecuteQueryRequest
};
use crate::schemas::error::{Error, Problem};
use crate::db::db_manager::{SharedDbManager};
use actix_web::{HttpResponse, Responder, get, post, web};
use rand::random;
use uuid::Uuid;

#[get("/queries")]
async fn get_queries(
    db_manager: SharedDbManager
) -> impl Responder
{
    println!("Fetching all QUERIES");

    let manager = db_manager.read().await;
    let queries = manager.get_queries();

    drop(manager);

    HttpResponse::Ok().json(queries)
}


#[get("/query/{query_id}")]
async fn get_query_info(
    query_id: web::Path<Uuid>,
    db_manager: SharedDbManager
) -> impl Responder
{
    println!("Fetching query with id: {}", query_id);

    let manager = db_manager.read().await;
    let query_details = manager.get_query_details(&query_id);

    drop(manager);

    return match query_details
    {
        Ok(q) => HttpResponse::Ok().json(q),
        Err(e) => HttpResponse::NotFound().json(
            Error::new(&format!("{}", e))
        )
    };
}

#[post("/query")]
async fn post_query(query: web::Json<ExecuteQueryRequest>) -> impl Responder
{
    let id = match query.query_definition()
    {
        AllowedQuery::SELECT_Q(select_q) => "select_id",
        AllowedQuery::COPY_Q(copy_q) => "copy_id",
    };
    
    HttpResponse::Ok().json(id)
}

