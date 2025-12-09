use crate::db::errors::DbError;
use crate::db::storage::metadata::{DbMetadata, TableMetadata};
use crate::schemas::error::MultipleProblemsError;
use crate::schemas::table::{ShallowTable, TableSchema};
use crate::schemas::query::{AllowedQuery, CopyQuery, Query, ShallowQuery, QueryResult};
use uuid::Uuid;
use tokio::sync::mpsc::{UnboundedSender};

/// Conn means Connection
type ConnId = Uuid;
type QueryId = Uuid;
type TableId = Uuid;
type WorkerId = usize;
type RowCount = usize;
type FlushResult = bool;
type ColName = String;
type LastColFileId = u16;

pub enum DbCmd
{
    Client(DbClientMsg),
    DbWorker(DbWorkerMsg),
    Shutdown,
}

pub enum DbWorkerMsg
{
    ExecQuery(WorkerId, QueryData),
    QueryCompleted(WorkerId, QueryCompletionMsg),
    QueryFailed(WorkerId, QueryFailureMsg),
    InternalError(WorkerId, QueryFailureMsg),
    Shutdown
}

pub enum DbClientMsg
{
    Register(ConnId, UnboundedSender<ResMsg>),
    GetTables(ConnId),
    GetTableDetails(ConnId, TableId),
    DeleteTable(ConnId, TableId),
    PutTable(ConnId, TableSchema),
    GetQueries(ConnId),
    GetQueryDetails(ConnId, QueryId),
    PostQuery(ConnId, AllowedQuery),
    GetQueryRes(ConnId, (QueryId, RowCount, FlushResult)),
    GetFailedQuery(ConnId, QueryId),
}

pub enum ResMsg
{
    ResTables(Vec<ShallowTable>),
    ResTableDetails(Result<TableSchema, DbError>),
    ResDeleteTable(Result<(), DbError>),
    ResPutTable(Result<Uuid, DbError>),
    ResQueries(Result<Vec<ShallowQuery>, DbError>),
    ResQueryDetails(Result<Query, DbError>),
    ResPostQuery(Result<Uuid, DbError>),
    ResQuery(Result<QueryResult, DbError>),
    ResFailedQuery(Result<MultipleProblemsError, DbError>),
}

pub enum DbMaintenanceMsg
{
    SaveMetadata(DbMetadata),
    DeleteTable(TableMetadata),
    Shutdown
}

#[derive(Debug)]
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

    pub fn table_id(&self) -> Uuid
    {
        self.table_id
    }

    pub fn query_id(&self) -> Uuid
    {
        self.query_id
    }

    /// Consumes self
    pub fn problems(self) -> MultipleProblemsError
    {
        self.problems
    }
}

pub enum WorkerMsgRes
{
    SelectRes(QueryResult),
    CopyRes(Vec<(ColName, LastColFileId)>)
}

pub struct QueryCompletionMsg
{
    query_id: Uuid,
    table_id: Uuid, 
    n_rows: i32,
    res: WorkerMsgRes,
}

impl QueryCompletionMsg
{
    pub fn new(
        query_id: Uuid,
        table_id: Uuid,
        n_rows: i32,
        res: WorkerMsgRes
    ) -> QueryCompletionMsg
    {
        QueryCompletionMsg { query_id, table_id, n_rows, res }
    }

    pub fn table_id(&self) -> Uuid
    {
        self.table_id
    }

    pub fn query_id(&self) -> Uuid
    {
        self.query_id
    }

    pub fn res(self) -> WorkerMsgRes
    {
        self.res
    }

    pub fn res_ref(&self) -> &WorkerMsgRes
    {
        &self.res
    }
}

pub trait BaseQueryDataInfo
{
    fn query_id(&self) -> Uuid;
    fn table_id(&self) -> Uuid;
}

pub enum QueryData
{
    SelectQ(SelectQData),
    CopyQ(CopyQData)
}

impl BaseQueryDataInfo for QueryData
{
    fn query_id(&self) -> Uuid
    {
        match self
        {
            Self::SelectQ(q) => q.query_id(),
            Self::CopyQ(q) => q.query_id(),
        }
    }

    fn table_id(&self) -> Uuid
    {
        match self
        {
            Self::SelectQ(q) => q.table_id(),
            Self::CopyQ(q) => q.table_id(),
        }
    }
}

// TODO: move below struct implementation to separate file 
#[derive(Debug)]
pub struct SelectQData
{
    query_id: Uuid,
    table_metadata: TableMetadata
}

impl BaseQueryDataInfo for SelectQData
{
    fn query_id(&self) -> Uuid{
        self.query_id
    }

    fn table_id(&self) -> Uuid {
        self.table_metadata.table_id()
    }

}

impl SelectQData
{
    pub fn new(query_id: Uuid, table_metadata: TableMetadata) -> SelectQData
    {
        SelectQData { query_id, table_metadata }
    }

    pub fn table_metadata(&self) -> &TableMetadata
    {
        &self.table_metadata
    }
    // TODO: These two func should be in trait
}

#[derive(Debug)]
pub struct CopyQData
{
    query_id: Uuid,
    copy_q: CopyQuery,
    table_metadata: TableMetadata
}

impl BaseQueryDataInfo for CopyQData
{
    fn query_id(&self) -> Uuid{
        self.query_id
    }

    fn table_id(&self) -> Uuid {
        self.table_metadata.table_id()
    }

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

    pub fn table_metadata(&self) -> &TableMetadata
    {
        &self.table_metadata
    }

    pub fn query_data(&self) -> &CopyQuery
    {
        &self.copy_q
    }
}