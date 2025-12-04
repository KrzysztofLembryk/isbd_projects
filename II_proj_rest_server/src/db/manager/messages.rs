use crate::db::errors::DbError;
use crate::db::storage::col_data::ColType;
use crate::db::storage::metadata::{DbMetadata, TableMetadata};
use crate::schemas::column::DataColumn;
use crate::schemas::table::{ShallowTable, TableSchema};
use crate::schemas::query::{AllowedQuery, CopyQuery, Query, SelectQuery, ShallowQuery};
use uuid::Uuid;
use tokio::sync::mpsc::{UnboundedSender};

/// Conn means Connection
type ConnId = Uuid;
type WorkerId = usize;

pub enum DbCmd
{
    Client(DbClientMsg),
    DbWorker(DbWorkerMsg),
    Shutdown,
}

pub enum DbWorkerMsg
{
    DoQuery(WorkerId, QueryData),
    QueryCompleted(WorkerId, QueryRes),
    Shutdown
}

pub enum DbClientMsg
{
    Register(ConnId, UnboundedSender<ResMsg>),
    GetTables(ConnId),
    GetTableDetails(ConnId, Uuid),
    DeleteTable(ConnId, Uuid),
    PutTable(ConnId, TableSchema),
    GetQueries(ConnId),
    GetQueryDetails(ConnId, Uuid),
    PostQuery(ConnId, AllowedQuery),
}

pub enum ResMsg
{
    ResTables(Result<Vec<ShallowTable>, DbError>),
    ResTableDetails(Result<TableSchema, DbError>),
    ResDeleteTable(Result<(), DbError>),
    ResPutTable(Result<Uuid, DbError>),
    ResQueries(Result<Vec<ShallowQuery>, DbError>),
    ResQueryDetails(Result<Query, DbError>),
    ResPostQuery(Result<Uuid, DbError>),
}

pub enum DbMaintenanceMsg
{
    SaveMetadata(DbMetadata),
    DeleteTable(TableMetadata),
    Shutdown
}

pub struct QueryRes
{
    query_id: Uuid,
    table_id: Uuid, 
    // If columns are None, this means it was CopyQuery
    // otherwise it was SelectQuery
    columns: Option<Vec<DataColumn>>,
}

pub enum QueryData
{
    SelectQ(SelectQData),
    CopyQ(CopyQData)
}
// TODO: move below struct implementation to separate file 
pub struct SelectQData
{
    id: Uuid,
    table_metadata: TableMetadata
}

impl SelectQData
{
    pub fn new(id: Uuid, table_metadata: TableMetadata) -> SelectQData
    {
        SelectQData { id, table_metadata }
    }
}
pub struct CopyQData
{
    copy_q: CopyQuery,
    table_metadata: TableMetadata
}

impl CopyQData
{
    pub fn new(copy_q: CopyQuery, table_metadata: TableMetadata) -> CopyQData
    {
        CopyQData { copy_q, table_metadata }
    }
}