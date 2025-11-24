use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use crate::schemas::table::{ShallowTable};

#[get("/tables")]
async fn get_tables() -> Result<impl Responder>
{
    let mut tables = vec![];
    tables.push(ShallowTable {table_id: "1", name: ""});
    HttpResponse::Ok().body(format!("YOOOLOOO, lecimy ztym, id: {}, joke: {}", info.user_id, info.joke))
}