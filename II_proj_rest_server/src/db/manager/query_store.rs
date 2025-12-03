use crate::schemas::query::{Query, QueryStatus};
use crate::db::errors::DbError;

use std::collections::VecDeque;
use std::collections::HashMap;
use uuid::Uuid;

pub struct QueryStore
{
    queries: HashMap<Uuid, Query>,
    query_queue: VecDeque<Uuid>, // stores order of queries to be executed
}

impl QueryStore
{
    pub fn new() -> QueryStore
    {
        QueryStore { queries: HashMap::new(), query_queue: VecDeque::new() }
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

    pub fn pop_pending_query(&mut self) -> Option<&Query>
    {
        if let Some(q_id) = self.query_queue.pop_front()
        {
            return self.queries.get(&q_id);
        }
        None
    }

    pub fn has_pending_queries(&self) -> bool
    {
        !self.query_queue.is_empty()
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
        Err(DbError::NotFound(format!("QueryStore::update_query_status: query with id: '{}'", q_id)))
    }

}