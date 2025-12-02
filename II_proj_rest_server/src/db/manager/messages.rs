use crate::db::errors::DbError;
use crate::db::storage::metadata::{DbMetadata, TableMetadata};
use crate::schemas::table::{ShallowTable, TableSchema};
use crate::schemas::query::{AllowedQuery, Query, ShallowQuery};
use uuid::Uuid;
use tokio::sync::mpsc::{UnboundedSender};

/// Conn means Connection
type ConnId = Uuid;

/// Db Command
pub enum DbCmd
{
    Register(ConnId, UnboundedSender<ResMsg>),
    GetTables(ConnId),
    GetTableDetails(ConnId, Uuid),
    DeleteTable(ConnId, Uuid),
    PutTable(ConnId, TableSchema),
    GetQueries(ConnId),
    GetQueryDetails(ConnId, Uuid),
    PostQuery(ConnId, AllowedQuery),
    Shutdown,
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

type DirPath = String;
pub enum DbMaintenanceMsg
{
    SaveMetadata(DbMetadata),
    DeleteTable(TableMetadata),
    Shutdown
}