use II_proj_rest_server::schemas::id_types::IdTypes;
use II_proj_rest_server::schemas::column::LogicalColType;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;

#[derive(Deserialize)]
struct Info 
{
    user_id: u32,
    joke: String,
}

#[get("/{user_id}/{joke}")]
async fn hello(info: web::Query<Info>) -> impl Responder
{

    HttpResponse::Ok().body(format!("YOOOLOOO, lecimy ztym, id: {}, joke: {}", info.user_id, info.joke))
}


#[actix_web::main]
async fn main() -> std::io::Result<()>
{
    HttpServer::new(|| {
        App::new()
            .service(hello)   
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
