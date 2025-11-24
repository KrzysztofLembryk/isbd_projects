use crate::schemas::column::{DataColumn};
use uuid::Uuid;

pub enum QueryStatus
{
    CREATED,
    PLANNING,
    RUNNING,
    COMPLETED,
    FAILED
}

pub enum AllowedQuery
{
    SELECT_Q(SelectQuery),
    COPY_Q(CopyQuery)
}

pub struct ShallowQuery
{
    query_id: Uuid,
    status: QueryStatus

}

pub struct Query
{
    query_id: Uuid,
    status: QueryStatus,
    is_res_available: bool,
    query_definition: AllowedQuery

}

pub struct ExecuteQueryRequest
{
    query_definition: AllowedQuery
}


/// - Server will read the file and insert all data into selected table.
/// - When number of columns in source and target doesn't match, user have to 
/// use "destinationColumns" property to specify which columns data should be 
/// inserted into.
pub struct CopyQuery
{
    src_filepath: String,
    dest_table_name: String,
    dest_columns: Option<Vec<String>>,
    does_csv_contain_header: bool,
}

pub struct SelectQuery
{
    table_name: String
}

pub struct QueryResult
{
    row_count: i32,
    columns: Vec<DataColumn>,
}