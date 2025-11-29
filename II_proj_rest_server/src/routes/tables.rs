use actix_web::{HttpResponse, Responder, delete, get, put, web};
use crate::db::db_manager::{DbManager};
use crate::db::errors::DbError;
use crate::db::storage::metadata_structs::DbMetadata;
use crate::schemas::table::{ShallowTable, TableSchema};
use crate::schemas::column::{Column};
use crate::schemas::error::{Error, Problem};
use uuid::Uuid;
use rand::random;

// Request body from HTTP request, and getting data to buffer with given SIZE
// https://actix.rs/docs/request/

#[get("/tables")]
async fn get_tables(
    db_manager: web::Data<tokio::sync::RwLock<DbManager>>
) -> impl Responder
{
    println!("Fetching ALL TABLES DETAILS");

    let manager = db_manager.read().await;
    let details = manager.get_tables();

    drop(manager);

    return match details
    {
        Ok(tables) => HttpResponse::Ok().json(tables),
        Err(e) => HttpResponse::NotFound().json(
            Error::new(&format!("{}", e))
        )
    };
}

#[get("/table/{table_id}")]
async fn get_table_details(
    table_id: web::Path<Uuid>, 
    db_manager: web::Data<tokio::sync::RwLock<DbManager>>
) -> impl Responder
{
    println!("Fetching DETAILS for table with id: {}", table_id);

    let manager = db_manager.read().await;
    let details = manager.get_table_details(&table_id);

    // We release lock right after reading data from manager
    drop(manager);

    return match details
    {
        Ok(table_schema) => HttpResponse::Ok().json(table_schema),
        Err(e) => HttpResponse::NotFound().json(
            Error::new(&format!("{}", e))
        )
    };
}

#[delete("/table/{table_id}")]
async fn delete_table(table_id: web::Path<String>) -> impl Responder
{
    println!("Fetching table with id: {}", table_id);

    if random::<bool>()
    {
        return HttpResponse::NotFound().json(
            Error::new(&format!("table with id: '{}' not found", table_id))
        )
    }

    HttpResponse::Ok().body(format!("table: '{}' removed successfully", table_id))
}

#[put("/table")]
async fn put_table(
    table_schema: web::Json<TableSchema>,
    db_manager: web::Data<tokio::sync::RwLock<DbManager>>
) -> impl Responder
{
    println!("Putting new table");

    let mut manager = db_manager.write().await;
    let result = manager.put_table(&table_schema.into_inner()).await;

    drop(manager);

    return match result
    {
        Ok(table_id) => HttpResponse::Ok().body(table_id.to_string()),
        Err(e) => {
            match e
            {
                DbError::InternalDbError(e) => 
                    HttpResponse::InternalServerError()
                        .json(Error::new(&format!("{}", e))),
                e => 
                    HttpResponse::BadRequest()
                        .json(Error::new(&format!("{}", e)))
            }
        }
    };
}