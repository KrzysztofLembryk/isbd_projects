use crate::schemas::column::{DataColumn};
use uuid::Uuid;
use serde;

#[derive(Clone, Copy, serde::Serialize)]
pub enum QueryStatus
{
    CREATED,
    PLANNING,
    RUNNING,
    COMPLETED,
    FAILED
}

#[derive(serde::Serialize)]
pub enum AllowedQuery
{
    SELECT_Q(SelectQuery),
    COPY_Q(CopyQuery)
}

#[derive(serde::Serialize)]
pub struct ShallowQuery
{
    #[serde(rename="queryId")]
    query_id: Uuid,
    status: QueryStatus
}

impl ShallowQuery
{
    pub fn new(id: &Uuid, status: QueryStatus) -> ShallowQuery
    {
        ShallowQuery { query_id: id.clone(), status: status}
    }
}

#[derive(serde::Serialize)]
pub struct Query
{
    #[serde(rename="queryId")]
    query_id: Uuid,

    status: QueryStatus,

    #[serde(rename="isResultAvailable")]
    is_res_available: bool,

    #[serde(rename="queryDefinition")]
    query_definition: AllowedQuery
}

#[derive(serde::Serialize)]
pub struct ExecuteQueryRequest
{
    #[serde(rename="queryDefinition")]
    query_definition: AllowedQuery
}


/// - Server will read the file and insert all data into selected table.
/// - When number of columns in source and target doesn't match, user have to 
/// use "destinationColumns" property to specify which columns data should be 
/// inserted into.
/// 
#[derive(serde::Serialize)]
pub struct CopyQuery
{
    #[serde(rename="sourceFilepath")]
    src_filepath: String,
    #[serde(rename="destinationTableName")]
    dest_table_name: String,
    #[serde(rename="destinationColumns")]
    dest_columns: Option<Vec<String>>,
    #[serde(rename="doesCsvContainHeader")]
    does_csv_contain_header: bool,
}

#[derive(serde::Serialize)]
pub struct SelectQuery
{
    #[serde(rename="tableName")]
    table_name: String
}

#[derive(serde::Serialize)]
pub struct QueryResult
{
    #[serde(rename="rowCount")]
    row_count: i32,
    columns: Vec<DataColumn>,
}