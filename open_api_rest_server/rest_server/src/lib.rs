#![allow(missing_docs, trivial_casts, unused_variables, unused_mut, unused_imports, unused_extern_crates, unused_attributes, non_camel_case_types)]
#![allow(clippy::derive_partial_eq_without_eq, clippy::disallowed_names)]

use async_trait::async_trait;
use futures::Stream;
use std::error::Error;
use std::collections::BTreeSet;
use std::task::{Poll, Context};
use swagger::{ApiError, ContextWrapper};
use serde::{Serialize, Deserialize};
use crate::server::Authorization;


type ServiceError = Box<dyn Error + Send + Sync + 'static>;

pub const BASE_PATH: &str = "";
pub const API_VERSION: &str = "1.0.0";

mod auth;
pub use auth::{AuthenticationApi, Claims};


#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum GetQueriesResponse {
    /// Array of queries submitted to the system
    ArrayOfQueriesSubmittedToTheSystem
    (Vec<models::ShallowQuery>)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum GetSystemInfoResponse {
    /// Basic information about the system
    BasicInformationAboutTheSystem
    (models::SystemInformation)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum SubmitQueryResponse {
    /// Query has been created successfully
    QueryHasBeenCreatedSuccessfully
    (String)
    ,
    /// Response used when more problems can occur in the system when processing request
    ResponseUsedWhenMoreProblemsCanOccurInTheSystemWhenProcessingRequest
    (models::MultipleProblemsError)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum GetQueryByIdResponse {
    /// Detailed Query description
    DetailedQueryDescription
    (models::Query)
    ,
    /// Generic error
    GenericError
    (models::Error)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum GetQueryErrorResponse {
    /// Response used when more problems can occur in the system when processing request
    ResponseUsedWhenMoreProblemsCanOccurInTheSystemWhenProcessingRequest
    (models::MultipleProblemsError)
    ,
    /// Generic error
    GenericError
    (models::Error)
    ,
    /// Generic error
    GenericError_2
    (models::Error)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum GetQueryResultResponse {
    /// Result of selected query
    ResultOfSelectedQuery
    (Vec<models::QueryResultInner>)
    ,
    /// Generic error
    GenericError
    (models::Error)
    ,
    /// Generic error
    GenericError_2
    (models::Error)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum CreateTableResponse {
    /// Table created successfully
    TableCreatedSuccessfully
    (String)
    ,
    /// Response used when more problems can occur in the system when processing request
    ResponseUsedWhenMoreProblemsCanOccurInTheSystemWhenProcessingRequest
    (models::MultipleProblemsError)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum GetTablesResponse {
    /// Array of tables in database
    ArrayOfTablesInDatabase
    (Vec<models::ShallowTable>)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum DeleteTableResponse {
    /// Table has been deleted successfully
    TableHasBeenDeletedSuccessfully
    ,
    /// Generic error
    GenericError
    (models::Error)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[must_use]
pub enum GetTableByIdResponse {
    /// Detailed Table description
    DetailedTableDescription
    (models::TableSchema)
    ,
    /// Generic error
    GenericError
    (models::Error)
}

/// API
#[async_trait]
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub trait Api<C: Send + Sync> {
    /// Get list of queries (optional in project 3, but useful). Use those IDs to get details by calling /query endpoint.
    async fn get_queries(
        &self,
        context: &C) -> Result<GetQueriesResponse, ApiError>;

    /// Get basic information about the system (e.g. version, uptime, etc.)
    async fn get_system_info(
        &self,
        context: &C) -> Result<GetSystemInfoResponse, ApiError>;

    /// Submit new query for execution
    async fn submit_query(
        &self,
        execute_query_request: models::ExecuteQueryRequest,
        context: &C) -> Result<SubmitQueryResponse, ApiError>;

    /// Get detailed status of selected query
    async fn get_query_by_id(
        &self,
        query_id: String,
        context: &C) -> Result<GetQueryByIdResponse, ApiError>;

    /// Get error of selected query (will be available only for queries in FAILED state)
    async fn get_query_error(
        &self,
        query_id: String,
        context: &C) -> Result<GetQueryErrorResponse, ApiError>;

    /// Get result of selected query (will be available only for SELECT queries after they are completed)
    async fn get_query_result(
        &self,
        query_id: String,
        get_query_result_request: Option<models::GetQueryResultRequest>,
        context: &C) -> Result<GetQueryResultResponse, ApiError>;

    /// Create new table in database
    async fn create_table(
        &self,
        table_schema: models::TableSchema,
        context: &C) -> Result<CreateTableResponse, ApiError>;

    /// Get list of tables with their accompaning IDs. Use those IDs to get details by calling /table endpoint.
    async fn get_tables(
        &self,
        context: &C) -> Result<GetTablesResponse, ApiError>;

    /// Delete selected table from database
    async fn delete_table(
        &self,
        table_id: String,
        context: &C) -> Result<DeleteTableResponse, ApiError>;

    /// Get detailed description of selected table
    async fn get_table_by_id(
        &self,
        table_id: String,
        context: &C) -> Result<GetTableByIdResponse, ApiError>;

}

/// API where `Context` isn't passed on every API call
#[async_trait]
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub trait ApiNoContext<C: Send + Sync> {

    fn context(&self) -> &C;

    /// Get list of queries (optional in project 3, but useful). Use those IDs to get details by calling /query endpoint.
    async fn get_queries(
        &self,
        ) -> Result<GetQueriesResponse, ApiError>;

    /// Get basic information about the system (e.g. version, uptime, etc.)
    async fn get_system_info(
        &self,
        ) -> Result<GetSystemInfoResponse, ApiError>;

    /// Submit new query for execution
    async fn submit_query(
        &self,
        execute_query_request: models::ExecuteQueryRequest,
        ) -> Result<SubmitQueryResponse, ApiError>;

    /// Get detailed status of selected query
    async fn get_query_by_id(
        &self,
        query_id: String,
        ) -> Result<GetQueryByIdResponse, ApiError>;

    /// Get error of selected query (will be available only for queries in FAILED state)
    async fn get_query_error(
        &self,
        query_id: String,
        ) -> Result<GetQueryErrorResponse, ApiError>;

    /// Get result of selected query (will be available only for SELECT queries after they are completed)
    async fn get_query_result(
        &self,
        query_id: String,
        get_query_result_request: Option<models::GetQueryResultRequest>,
        ) -> Result<GetQueryResultResponse, ApiError>;

    /// Create new table in database
    async fn create_table(
        &self,
        table_schema: models::TableSchema,
        ) -> Result<CreateTableResponse, ApiError>;

    /// Get list of tables with their accompaning IDs. Use those IDs to get details by calling /table endpoint.
    async fn get_tables(
        &self,
        ) -> Result<GetTablesResponse, ApiError>;

    /// Delete selected table from database
    async fn delete_table(
        &self,
        table_id: String,
        ) -> Result<DeleteTableResponse, ApiError>;

    /// Get detailed description of selected table
    async fn get_table_by_id(
        &self,
        table_id: String,
        ) -> Result<GetTableByIdResponse, ApiError>;

}

/// Trait to extend an API to make it easy to bind it to a context.
pub trait ContextWrapperExt<C: Send + Sync> where Self: Sized
{
    /// Binds this API to a context.
    fn with_context(self, context: C) -> ContextWrapper<Self, C>;
}

impl<T: Api<C> + Send + Sync, C: Clone + Send + Sync> ContextWrapperExt<C> for T {
    fn with_context(self: T, context: C) -> ContextWrapper<T, C> {
         ContextWrapper::<T, C>::new(self, context)
    }
}

#[async_trait]
impl<T: Api<C> + Send + Sync, C: Clone + Send + Sync> ApiNoContext<C> for ContextWrapper<T, C> {
    fn context(&self) -> &C {
        ContextWrapper::context(self)
    }

    /// Get list of queries (optional in project 3, but useful). Use those IDs to get details by calling /query endpoint.
    async fn get_queries(
        &self,
        ) -> Result<GetQueriesResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().get_queries(&context).await
    }

    /// Get basic information about the system (e.g. version, uptime, etc.)
    async fn get_system_info(
        &self,
        ) -> Result<GetSystemInfoResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().get_system_info(&context).await
    }

    /// Submit new query for execution
    async fn submit_query(
        &self,
        execute_query_request: models::ExecuteQueryRequest,
        ) -> Result<SubmitQueryResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().submit_query(execute_query_request, &context).await
    }

    /// Get detailed status of selected query
    async fn get_query_by_id(
        &self,
        query_id: String,
        ) -> Result<GetQueryByIdResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().get_query_by_id(query_id, &context).await
    }

    /// Get error of selected query (will be available only for queries in FAILED state)
    async fn get_query_error(
        &self,
        query_id: String,
        ) -> Result<GetQueryErrorResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().get_query_error(query_id, &context).await
    }

    /// Get result of selected query (will be available only for SELECT queries after they are completed)
    async fn get_query_result(
        &self,
        query_id: String,
        get_query_result_request: Option<models::GetQueryResultRequest>,
        ) -> Result<GetQueryResultResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().get_query_result(query_id, get_query_result_request, &context).await
    }

    /// Create new table in database
    async fn create_table(
        &self,
        table_schema: models::TableSchema,
        ) -> Result<CreateTableResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().create_table(table_schema, &context).await
    }

    /// Get list of tables with their accompaning IDs. Use those IDs to get details by calling /table endpoint.
    async fn get_tables(
        &self,
        ) -> Result<GetTablesResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().get_tables(&context).await
    }

    /// Delete selected table from database
    async fn delete_table(
        &self,
        table_id: String,
        ) -> Result<DeleteTableResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().delete_table(table_id, &context).await
    }

    /// Get detailed description of selected table
    async fn get_table_by_id(
        &self,
        table_id: String,
        ) -> Result<GetTableByIdResponse, ApiError>
    {
        let context = self.context().clone();
        self.api().get_table_by_id(table_id, &context).await
    }

}


#[cfg(feature = "client")]
pub mod client;

// Re-export Client as a top-level name
#[cfg(feature = "client")]
pub use client::Client;

#[cfg(feature = "server")]
pub mod server;

// Re-export router() as a top-level name
#[cfg(feature = "server")]
pub use self::server::Service;

#[cfg(feature = "server")]
pub mod context;

pub mod models;

#[cfg(any(feature = "client", feature = "server"))]
pub(crate) mod header;
