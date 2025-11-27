use crate::schemas::column::{Column};
use serde;
use uuid::Uuid;
use std::fmt;
use validator::{Validate, ValidationError};

#[derive(serde::Serialize, serde::Deserialize, Validate)]
pub struct TableSchema
{
    #[validate(length(min=1))]
    name: String,
    columns: Vec<Column>
}

impl fmt::Display for TableSchema
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result 
    {
        write!(f, "name: {}", self.name)?;
        write!(f, "cols: {:?}", self.columns)
    }
}

impl TableSchema
{
    pub fn new(name: &str) -> TableSchema
    {
        TableSchema { name: String::from(name), columns: Vec::new()}
    }

    pub fn push_col(&mut self, col: &Column)
    {
        self.columns.push(col.clone());
    }

    pub fn name(&self) -> &str
    {
        &self.name
    }

    pub fn columns(&self) -> &Vec<Column>
    {
        &self.columns
    }
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

