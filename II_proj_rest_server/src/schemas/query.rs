use crate::schemas::column::{DataColumn};
use uuid::Uuid;
use serde;
use serde::ser::Serialize;

#[derive(Clone, Copy, serde::Serialize)]
pub enum QueryStatus
{
    CREATED,
    PLANNING,
    RUNNING,
    COMPLETED,
    FAILED
}

#[derive(Clone, serde::Deserialize)]
pub enum AllowedQuery
{
    SELECT_Q(SelectQuery),
    COPY_Q(CopyQuery)
}

// We need to implement serialization since we dont want our response to contain
// SELECT_Q or COPY_Q in json
impl Serialize for AllowedQuery
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer 
    {
        match self     
        {
            AllowedQuery::COPY_Q(q) => q.serialize(serializer),
            AllowedQuery::SELECT_Q(q) => q.serialize(serializer),
        }
    }
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
    pub fn new(id: Uuid, status: QueryStatus) -> ShallowQuery
    {
        ShallowQuery { query_id: id, status: status}
    }
}

#[derive(serde::Serialize, Clone)]
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

impl Query
{
    pub fn new(
        status: QueryStatus, 
        is_res_available: bool, 
        query_definition: AllowedQuery
    ) -> Query
    {
        let new_uuid = Uuid::new_v4();

        Query { 
            query_id: new_uuid, 
            status: status, 
            is_res_available: is_res_available, 
            query_definition 
        }
    }

    pub fn status(&self) -> QueryStatus
    {
        self.status.clone()
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ExecuteQueryRequest
{
    #[serde(rename="queryDefinition")]
    pub query_definition: AllowedQuery
}


/// - Server will read the file and insert all data into selected table.
/// - When number of columns in source and target doesn't match, user have to 
/// use "destinationColumns" property to specify which columns data should be 
/// inserted into.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
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

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct SelectQuery
{
    #[serde(rename="tableName")]
    table_name: String
}

impl SelectQuery
{
    pub fn new(table_name: &str) -> SelectQuery
    {
        SelectQuery{table_name: String::from(table_name)}
    }
}

#[derive(serde::Serialize)]
pub struct QueryResult
{
    #[serde(rename="rowCount")]
    row_count: i32,
    columns: Vec<DataColumn>,
}