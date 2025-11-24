use actix_web::{HttpResponse, Responder, get};
use crate::schemas::query::{ShallowQuery, QueryStatus};
use uuid::Uuid;

#[get("/queries")]
async fn get_queries() -> impl Responder
{
    let mut queries = vec![];

    queries.push(ShallowQuery::new(&Uuid::new_v4(), QueryStatus::CREATED));
    queries.push(ShallowQuery::new(&Uuid::new_v4(), QueryStatus::PLANNING));
    queries.push(ShallowQuery::new(&Uuid::new_v4(), QueryStatus::RUNNING));

    HttpResponse::Ok().json(queries)
}


