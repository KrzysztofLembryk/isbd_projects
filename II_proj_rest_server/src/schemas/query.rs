use crate::schemas::column::{DataColumn};
use uuid::Uuid;
use serde;
use serde::ser::Serialize;

#[derive(Clone, Copy, serde::Serialize, Debug)]
pub enum QueryStatus
{
    CREATED,
    PLANNING,
    RUNNING,
    COMPLETED,
    FAILED
}

#[derive(Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum AllowedQuery
{
    SelectQ(SelectQuery),
    CopyQ(CopyQuery)
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
            AllowedQuery::CopyQ(q) => q.serialize(serializer),
            AllowedQuery::SelectQ(q) => q.serialize(serializer),
        }
    }
}

#[derive(serde::Serialize, Debug)]
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
        query_definition: AllowedQuery
    ) -> Query
    {
        let query_id = Uuid::new_v4();
        Query { 
            query_id, 
            status: QueryStatus::CREATED, 
            is_res_available: false, 
            query_definition 
        }
    }

    pub fn id(&self) -> &Uuid
    {
        &self.query_id
    }

    pub fn status(&self) -> QueryStatus
    {
        self.status.clone()
    }

    pub fn table_name(&self) -> &str
    {
        self.query_definition.table_name()
    }

    pub fn query_def(&self) -> &AllowedQuery
    {
        &self.query_definition
    }

    pub fn update_status(&mut self, new_status: QueryStatus)
    {
        match new_status
        {
            QueryStatus::COMPLETED => {
                self.is_res_available = true;
                self.status = new_status;
            },
            _ => {
                self.status = new_status;
            }

        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ExecuteQueryRequest
{
    #[serde(rename="queryDefinition")]
    query_definition: AllowedQuery
}

impl ExecuteQueryRequest
{
    pub fn query_definition(&self) -> AllowedQuery
    {
        self.query_definition.clone()
    }
}


/// - Server will read the file and insert all data into selected table.
/// - When number of columns in source and target doesn't match, user have to 
/// use "destinationColumns" property to specify which columns data should be 
/// inserted into.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
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

    pub fn table_name(&self) -> &str
    {
        &self.table_name
    }
}

pub trait QueryTableName {
    fn table_name(&self) -> &str;
}

impl QueryTableName for CopyQuery {
    fn table_name(&self) -> &str {
        &self.dest_table_name
    }
}

impl QueryTableName for SelectQuery {
    fn table_name(&self) -> &str {
        &self.table_name
    }
}

impl QueryTableName for AllowedQuery {
    fn table_name(&self) -> &str {
        match self {
            AllowedQuery::SelectQ(q) => q.table_name(),
            AllowedQuery::CopyQ(q) => q.table_name(),
        }
    }
}



#[derive(serde::Serialize, Clone)]
pub struct QueryResult
{
    #[serde(rename="rowCount")]
    row_count: i32,
    columns: Vec<DataColumn>,
}

impl QueryResult
{
    pub fn new(row_count: i32, columns: Vec<DataColumn>) -> QueryResult
    {
        QueryResult { row_count, columns }
    }

    pub fn push_col_data(&mut self, col_data: DataColumn)
    {
        self.columns.push(col_data);
    }
}