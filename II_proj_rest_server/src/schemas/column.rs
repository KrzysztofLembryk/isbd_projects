use serde;

#[derive(serde::Serialize)]
pub enum LogicalColType
{
    INT64,
    VARCHAR
}

pub enum DataColumn
{
    Int64(Int64Column),
    Varchar(VarcharColumn),
}

pub struct Column
{
    c_name: String,
    c_type: LogicalColType 
}

pub struct Int64Column
{
    values: Option<Vec<i64>>
}

pub struct VarcharColumn
{
    values: Option<Vec<String>>
}
