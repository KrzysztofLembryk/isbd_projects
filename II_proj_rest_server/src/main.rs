use II_proj_rest_server::routes::tables::{get_tables, get_table_info, put_table, delete_table};
use II_proj_rest_server::routes::queries::{get_queries};
use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()>
{
    HttpServer::new(|| {
        App::new()
            .service(get_tables)
            .service(get_table_info)
            .service(put_table)
            .service(delete_table)
            .service(get_queries)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
