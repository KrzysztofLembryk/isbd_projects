use crate::db::errors::DbError;
use crate::db::storage::col_data::ColType;
use crate::db::storage::metadata::{DbMetadata, TableMetadata};
use crate::schemas::column::DataColumn;
use crate::schemas::error::MultipleProblemsError;
use crate::schemas::table::{ShallowTable, TableSchema};
use crate::schemas::query::{AllowedQuery, CopyQuery, Query, SelectQuery, ShallowQuery, QueryResult};
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
    QueryCompleted(WorkerId, QueryComlpetionMsg),
    QueryFailed(WorkerId, QueryFailureMsg),
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

pub struct QueryFailureMsg
{
    query_id: Uuid,
    table_id: Uuid, 
    problems: MultipleProblemsError,
}

impl QueryFailureMsg
{
    pub fn new(
        query_id: Uuid,
        table_id: Uuid,
        problems: MultipleProblemsError
    ) -> QueryFailureMsg
    {
        QueryFailureMsg { query_id, table_id, problems }
    }
}

pub struct QueryComlpetionMsg
{
    query_id: Uuid,
    table_id: Uuid, 
    // If is None, this means it was CopyQuery
    // otherwise it was SelectQuery
    res: Option<QueryResult>,
}

impl QueryComlpetionMsg
{
    pub fn new(
            query_id: Uuid,
            table_id: Uuid,
            res: Option<QueryResult>
        ) -> QueryComlpetionMsg
        {
            QueryComlpetionMsg { query_id, table_id, res }
        }
}

pub enum QueryData
{
    SelectQ(SelectQData),
    CopyQ(CopyQData)
}
// TODO: move below struct implementation to separate file 
#[derive(Debug)]
pub struct SelectQData
{
    query_id: Uuid,
    table_metadata: TableMetadata
}

impl SelectQData
{
    pub fn new(query_id: Uuid, table_metadata: TableMetadata) -> SelectQData
    {
        SelectQData { query_id, table_metadata }
    }

    // TODO: These two func should be in trait
    pub fn query_id(&self) -> &Uuid
    {
        &self.query_id
    }

    pub fn table_metadata(&self) -> &TableMetadata
    {
        &self.table_metadata
    }
}

#[derive(Debug)]
pub struct CopyQData
{
    query_id: Uuid,
    copy_q: CopyQuery,
    table_metadata: TableMetadata
}

impl CopyQData
{
    pub fn new(
        query_id: Uuid, 
        copy_q: CopyQuery, 
        table_metadata: TableMetadata
    ) -> CopyQData
    {
        CopyQData { query_id, copy_q, table_metadata }
    }

    pub fn query_id(&self) -> &Uuid
    {
        &self.query_id
    }

    pub fn table_metadata(&self) -> &TableMetadata
    {
        &self.table_metadata
    }
}