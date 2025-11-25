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
use actix_web::{HttpResponse, Responder, get, post, web};
use uuid::Uuid;
use rand::random;

#[get("/queries")]
async fn get_queries() -> impl Responder
{
    let mut queries = vec![];

    queries.push(ShallowQuery::new(QueryStatus::CREATED));
    queries.push(ShallowQuery::new(QueryStatus::PLANNING));
    queries.push(ShallowQuery::new(QueryStatus::RUNNING));

    HttpResponse::Ok().json(queries)
}


#[get("/query/{query_id}")]
async fn get_query_info(query_id: web::Path<String>) -> impl Responder
{
    println!("Fetching query with id: {}", query_id);

    if random::<bool>()
    {
        return HttpResponse::NotFound().json(
            Error::new(&format!("query with id: '{}' not found", query_id))
        );
    }

    let query = Query::new(
        QueryStatus::COMPLETED, 
        true, 
        AllowedQuery::SELECT_Q(SelectQuery::new("table_1")
    ));


    HttpResponse::Ok().json(query)
}


#[post("/query")]
async fn post_query(query: web::Json<ExecuteQueryRequest>) -> impl Responder
{
    if random::<bool>()
    {
        return HttpResponse::BadRequest().json(
            Problem::new(&Error::new("Couldnt post query from request"), "post_query endpoint, random context")
        );
    }
    let id = match &query.query_definition
    {
        AllowedQuery::SELECT_Q(q) => "select_id",
        AllowedQuery::COPY_Q(q) => "copy_id",
    };
    
    HttpResponse::Ok().json(id)
}