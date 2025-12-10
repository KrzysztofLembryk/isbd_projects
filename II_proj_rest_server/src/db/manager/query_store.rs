use crate::schemas::query::{AllowedQuery, Query, QueryResult, QueryStatus, QueryType};
use crate::db::errors::DbError;
use crate::schemas::error::{MultipleProblemsError};

use std::collections::VecDeque;
use std::collections::HashMap;
use uuid::Uuid;

use log::{debug};

pub struct QueryStore
{
    queries: HashMap<Uuid, Query>,
    query_queue: VecDeque<Uuid>, // stores order of queries to be executed
    queries_results: HashMap<Uuid, QueryResult>,
    queries_failed: HashMap<Uuid, MultipleProblemsError>,
}

impl QueryStore
{
    pub fn new() -> QueryStore
    {
        QueryStore { 
            queries: HashMap::new(), 
            query_queue: VecDeque::new(),
            queries_results: HashMap::new(),
            queries_failed: HashMap::new(),
        }
    }

    pub fn get_query_mut_ref(
        &mut self, 
        query_id: Uuid
    ) -> Result<&mut Query, DbError>
    {
        match self.queries.get_mut(&query_id)
        {
            Some(v) => return Ok(v),
            None => {
                return Err(
                    DbError::InternalDbError(
                        format!("QueryStore::get_query_mut_ref - no query with id: {} exists in query store", query_id)
                    )
                );
            }
        }
    }

    pub fn queries(&self) -> &HashMap<Uuid, Query>
    {
        &self.queries
    }

    pub fn get_query_type(&self, query_id: &Uuid) -> Result<QueryType, DbError>
    {
        match self.queries.get(query_id)
        {
            Some(q) => {
                match q.query_def()
                {
                    AllowedQuery::CopyQ(_) => return Ok(QueryType::CopyQuery),
                    AllowedQuery::SelectQ(_) => return Ok(QueryType::SelectQuery),
                }
            }
            None => return Err(
                DbError::InternalDbError(
                    format!("QueryStore::get_query_type - query with id: '{}' doesnt exist", query_id)
                )
            )
        }
    }

    pub fn get_query_table_name(&self, query_id: &Uuid) -> Result<&str, DbError>
    {
        match self.queries.get(query_id)
        {
            Some(q) => return Ok(q.table_name()),
            None => return Err(
                DbError::InternalDbError(
                    format!("QueryStore::get_query_table_name - query with id: '{}' doesnt exist", query_id)
                )
            )
        }
    }

    pub fn insert_query(&mut self, q: Query) -> Result<(), DbError>
    {
        let q_id = q.id().clone();

        if self.queries.contains_key(&q_id)
        {
            return Err(DbError::Other(format!("QueryStore::push_query: Query with id: {} already exists", q_id)));
        }
        self.queries.insert(q_id.clone(), q);

        Ok(())
    }

    pub fn schedule_for_execution(&mut self, q_id: &Uuid)
    {
        self.query_queue.push_back(*q_id);
    }

    pub fn pop_pending_query(&mut self) -> Option<Uuid>
    {
        return self.query_queue.pop_front();
    }

    pub fn store_query_result(&mut self, q_id: &Uuid, q_res: QueryResult)
    {
        let _ = self.queries_results.insert(*q_id, q_res);
    }

    pub fn store_query_failure(&mut self, q_id: &Uuid, e: MultipleProblemsError)
    {
        let _ = self.queries_failed.insert(*q_id, e);
    }

    pub fn get_query_result(&self, q_id: &Uuid) -> Result<&QueryResult, DbError>
    {
        if let Some(res) = self.queries_results.get(q_id)
        {
            return Ok(res);
        }
        Err(DbError::NotFound(format!("There is no result for Query: '{}'", q_id)))
    }

    pub fn get_query_failure(
        &self, 
        q_id: &Uuid
    ) -> Result<MultipleProblemsError, DbError>
    {
        if let Some(res) = self.queries_failed.get(q_id)
        {
            return Ok(res.clone());
        }
        else
        {
            if let Some(_) = self.queries.get(q_id)
            {
                return Err(DbError::NotFound(format!("Query: '{}' hasn't FAILED", q_id)));
            }
            else
            {
                return Err(DbError::NotFound(format!("Query: '{}' NOT FOUND in db", q_id)));
            }
        }
    }

    pub fn remove_query_res(&mut self, query_id: &Uuid)
    {
        let _ = self.queries_results.remove(query_id);
    }

    pub fn update_query_status(
        &mut self, 
        q_id: &Uuid, 
        new_status: QueryStatus
    ) -> Result<(), DbError>
    {
        if let Some(q) = self.queries.get_mut(q_id)
        {
            q.update_status(new_status);
            return Ok(());
        }
        Err(DbError::InternalDbError(format!("QueryStore::update_query_status: query with id: '{}'", q_id)))
    }

    pub fn check_if_query_is_copy(
        &self, 
        query_id: &Uuid
    ) -> Result<bool, DbError>
    {
        let query = match self.queries.get(query_id) {
            Some(q) => q,
            None => {
                return Err(
                    DbError::InternalDbError(
                        format!("QueryStore::check_if_query_is_copy - query with id '{}' does not exist in QueryStore, corrupted db state", query_id)
                    )
                );
            }
        };
        match query.query_def()
        {
            AllowedQuery::CopyQ(_) => {
                return Ok(true);
            }
            AllowedQuery::SelectQ(_) => {
                return Ok(false);
            }
        }
    }
}