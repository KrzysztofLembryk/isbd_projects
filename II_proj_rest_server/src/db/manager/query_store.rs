use crate::schemas::query::{Query, QueryResult, QueryStatus};
use crate::db::errors::DbError;
use crate::schemas::error::{MultipleProblemsError};

use std::collections::VecDeque;
use std::collections::HashMap;
use actix_web::http::header::q;
use uuid::Uuid;

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

    pub fn queries(&self) -> &HashMap<Uuid, Query>
    {
        &self.queries
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

    pub fn pop_pending_query(&mut self) -> Option<&mut Query>
    {
        if let Some(q_id) = self.query_queue.pop_front()
        {
            return self.queries.get_mut(&q_id);
        }
        None
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
        Err(DbError::NotFound(format!("Query: '{}' hasn't FAILED", q_id)))
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

}