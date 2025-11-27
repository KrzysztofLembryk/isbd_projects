use serde;
use std::fmt;
use crate::db::constants::LogicalColType;

#[derive(serde::Serialize)]
pub enum DataColumn
{
    Int64(Int64Column),
    Varchar(VarcharColumn),
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct Column
{
    #[serde(rename = "name")]
    c_name: String,
    #[serde(rename = "type")]
    c_type: LogicalColType 
}

impl Column
{
    pub fn new(name: &str, c_type: &LogicalColType) -> Column
    {
        Column { c_name: String::from(name), c_type: c_type.clone() }
    }

    pub fn new_int(name: &str) -> Column
    {
        Column { c_name: String::from(name), c_type: LogicalColType::INT64 }
    }

    pub fn new_varchar(name: &str) -> Column
    {
        Column { c_name: String::from(name), c_type: LogicalColType::VARCHAR }
    }

    pub fn c_name(&self) -> &str
    {
        &self.c_name
    }

    pub fn c_type(&self) -> LogicalColType
    {
        self.c_type
    }
}

impl fmt::Display for Column {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result 
    {
        write!(f, "Column(name: {}, type: {:?})", self.c_name, self.c_type)
    }
}


#[derive(serde::Serialize)]
pub struct Int64Column
{
    values: Option<Vec<i64>>
}

#[derive(serde::Serialize)]
pub struct VarcharColumn
{
    values: Option<Vec<String>>
}
