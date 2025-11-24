use crate::schemas::column::{Column, LogicalColType};
use serde;
use uuid::Uuid;

pub struct TableSchema
{
    name: String,
    columns: Vec<Column>
}

#[derive(serde::Serialize)]
pub struct ShallowTable
{
    #[serde(rename = "tableId")]
    table_id: Uuid,
    name: String,
}

impl ShallowTable
{
    pub fn new(table_id: &Uuid, name: &str) -> ShallowTable
    {
        ShallowTable { table_id: table_id.clone(), name: String::from(name) }
    }
}

