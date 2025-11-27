use II_proj_rest_server::routes::tables::{get_tables, get_table_details, put_table, delete_table};
use II_proj_rest_server::routes::queries::{get_queries, get_query_info, post_query};
use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()>
{
    // 1. We initialize db_manager
    // 2. We initialize our Database (i.e. DbMetadata inside db_manager)
    // 3. We create Arc<RwLock<db_manager>>
    //      - RwLock - read write lock, allows many readers, but one writer
    //      - Arc - since we want to be able to share db_manager between Actix 
    //              threads
    HttpServer::new(|| {
        App::new()
            .service(get_tables)
            .service(get_table_details)
            .service(delete_table)
            .service(put_table)
            .service(get_queries)
            .service(get_query_info)
            .service(post_query)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
