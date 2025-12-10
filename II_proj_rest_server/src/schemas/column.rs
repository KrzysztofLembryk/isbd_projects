use serde;
use std::fmt;
use crate::db::constants::LogicalColType;
use validator::{Validate};

#[derive(serde::Serialize, Clone, Debug)]
pub enum DataColumn
{
    Int64(Int64Column),
    Varchar(VarcharColumn),
}

impl DataColumn
{
    pub fn clear_batch(&mut self)
    {
        match self
        {
            DataColumn::Int64(i_c) => i_c.clear(),
            DataColumn::Varchar(v_c) => v_c.clear(),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug, Validate)]
pub struct Column
{
    #[serde(rename = "name")]
    #[validate(length(min=1, max=255))]
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


#[derive(serde::Serialize, Clone, Debug)]
pub struct Int64Column
{
    values: Vec<i64>
}

impl Int64Column
{
    pub fn new(values: Vec<i64>) -> Int64Column
    {
        Int64Column { values }
    }

    pub fn values(&self) -> &Vec<i64>
    {
        &self.values
    }

    pub fn push(&mut self, val: i64)
    {
        self.values.push(val);
    }

    pub fn clear(&mut self)
    {
        self.values.clear();
    }
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct VarcharColumn
{
    values: Vec<String>
}

impl VarcharColumn
{
    pub fn new(values: Vec<String>) -> VarcharColumn
    {
        VarcharColumn { values }
    }

    pub fn values(&self) -> &Vec<String>
    {
        &self.values
    }

    pub fn push(&mut self, val: &str)
    {
        self.values.push(String::from(val));
    }

    pub fn clear(&mut self)
    {
        self.values.clear();
    }
}
