use actix_web::{HttpResponse, Responder, delete, get, put, web};
use crate::schemas::table::{ShallowTable, TableSchema};
use crate::schemas::column::{Column};
use crate::schemas::error::{Error, Problem};
use uuid::Uuid;
use rand::random;

// Request body from HTTP request, and getting data to buffer with given SIZE
// https://actix.rs/docs/request/

#[get("/tables")]
async fn get_tables() -> impl Responder
{
    if random::<bool>()
    {
        return HttpResponse::InternalServerError().body("Random err occured");
    }

    let mut tables = vec![];

    tables.push(ShallowTable::new(&Uuid::new_v4(), "table1"));
    tables.push(ShallowTable::new(&Uuid::new_v4(), "table2"));
    tables.push(ShallowTable::new(&Uuid::new_v4(), "table3"));

    HttpResponse::Ok().json(tables)
}

#[get("/table/{table_id}")]
async fn get_table_info(table_id: web::Path<String>) -> impl Responder
{
    println!("Fetching table with id: {}", table_id);

    if random::<bool>()
    {
        return HttpResponse::NotFound().json(
            Error::new(&format!("table with id: '{}' not found", table_id))
        )
    }

    let mut table_sch = TableSchema::new("table_schema_1");

    table_sch.push_col(&Column::new_int("col_int1"));
    table_sch.push_col(&Column::new_varchar("col_varchar1"));
    table_sch.push_col(&Column::new_int("col_int2"));

    HttpResponse::Ok().json(table_sch)
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
async fn put_table(table_schema: web::Json<TableSchema>) -> impl Responder
{
    if random::<bool>()
    {
        return HttpResponse::BadRequest().json(
            Problem::new(&Error::new("Couldnt create table from schema"), "put_table endpoint, random context")
        );
    }

    let new_uuid = Uuid::new_v4();
    println!("PUT /table --- got table schema:");
    println!("uuid: {}", new_uuid);
    println!("{}", table_schema);

    HttpResponse::Ok().body("table was created")
}