use self::super::{DbManager, DbCmd};
use crate::schemas::table::TableSchema;
use crate::schemas::query::{AllowedQuery, SelectQuery, CopyQuery, QueryStatus};
use crate::schemas::column::Column;
use crate::db::manager::messages::DbWorkerMsg;
use crate::db::constants::LogicalColType;
use tempfile::TempDir;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

const N_TABLES: usize = 2;
const USERS_TABLE_NAME: &str = "users";
const ORDERS_TABLE_NAME: &str = "orders";
const NBR_OF_DB_WORKERS: usize = 2;

#[cfg(test)]
mod db_manager {
    use crate::db::manager::messages::WorkerMsgRes;

    use super::*;
    
    #[tokio::test]
    async fn get_tables_correct()
    {
        let (db_manager, _temp_dir, _tx_db, _rx_db) = setup_test_db_manager(2).await;
        
        let tables = db_manager.get_tables();

        assert!(tables.is_ok());

        let tables_vec = tables.unwrap();
        
        // Check correct number of tables
        assert_eq!(tables_vec.len(), N_TABLES);
        
        // Collect table names
        let table_names: Vec<String> = tables_vec.iter()
            .map(|t| t.name())
            .collect();
        
        // Verify both expected tables are present
        assert!(table_names.contains(&USERS_TABLE_NAME.to_string()));
        assert!(table_names.contains(&ORDERS_TABLE_NAME.to_string()));
        
        db_manager.shutdown().await.expect("DbManager::Shutdown failed in test");
    }

    #[tokio::test]
    async fn post_query_select_success()
    {
        let (mut db_manager, _temp_dir, _tx_db, mut rx_db) = setup_test_db_manager(NBR_OF_DB_WORKERS).await;
        
        let select_query = AllowedQuery::SelectQ(
            SelectQuery::new(USERS_TABLE_NAME)
        );
        
        let result = db_manager.post_query(select_query);
        
        assert!(result.is_ok());
        let query_id = result.unwrap();
        
        // Verify query was added to query store
        let query_details = db_manager.get_query_details(&query_id);
        assert!(query_details.is_ok());
        
        let query = query_details.unwrap();
        assert!(matches!(query.status(), QueryStatus::CREATED | QueryStatus::RUNNING | QueryStatus::COMPLETED));
        
        // Listen for worker responses
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            rx_db.recv()
        ).await;
        
        assert!(response.is_ok(), "Timeout waiting for worker response");
        
        let db_cmd = response.unwrap();

        assert!(db_cmd.is_some(), "Expected worker response, got None");
        
        match db_cmd.unwrap() {
            DbCmd::DbWorker(worker_msg) => {
                match worker_msg {
                    DbWorkerMsg::QueryCompleted(worker_id, completion_msg) => {
                        // Verify the query ID matches
                        assert_eq!(completion_msg.query_id(), query_id);
                        
                        // Verify worker ID is valid
                        assert!(worker_id < 2, "Worker ID should be less than number of workers");
                        
                        // Verify result is present for SELECT query
                        // assert!(completion_msg.res() == WorkerMsgRes::, "SELECT query should return results");
                    }
                    DbWorkerMsg::QueryFailed(worker_id, failure_msg) => {
                        panic!("Query unexpectedly failed: worker_id={}, query_id={:?}", worker_id, failure_msg.query_id());
                    }
                    _ => {
                        panic!("Unexpected worker message type");
                    }
                }
            }
            _ => {
                panic!("Expected DbWorker command, got different command type");
            }
        }
        
        db_manager.shutdown().await.expect("DbManager::Shutdown failed in test");
    }
}

async fn setup_test_db_manager(
        nbr_of_db_workers: usize
) -> (DbManager, TempDir, UnboundedSender<DbCmd>, UnboundedReceiver<DbCmd>) 
{
    // Create temporary directory (automatically cleaned up when dropped)
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_dir = temp_dir.path().join("db_data");
    let metadata_file = temp_dir.path().join("metadata");
    
    // Convert to string
    let db_dir_str = db_dir.to_str().unwrap();
    let metadata_str = metadata_file.to_str().unwrap();
    
    // Create the db directory
    tokio::fs::create_dir_all(&db_dir).await.expect("Failed to create db dir");
    let (tx_db, rx_db) = unbounded_channel::<DbCmd>();
    let mut db_manager = 
        DbManager::new(
            tx_db.clone(), 
            db_dir_str, 
            metadata_str,
            nbr_of_db_workers
        )
        .await
        .expect("creating db_manager failed in test");

    db_manager.put_table(&users_table_schema())
        .expect("Failed to create users table");
    db_manager.put_table(&orders_table_schema())
        .expect("Failed to create orders table");

    // Return TempDir so it doesn't get dropped here
    (db_manager, temp_dir, tx_db, rx_db)
}
        
fn users_table_schema() -> TableSchema
{
    let mut users_schema = TableSchema::new(USERS_TABLE_NAME);
    users_schema.push_col(&Column::new(
        "id", 
        &LogicalColType::INT64)
    );
    users_schema.push_col(&Column::new(
        "username", 
        &LogicalColType::VARCHAR)
    );
    users_schema.push_col(&Column::new(
        "age", 
        &LogicalColType::INT64)
    );
    users_schema
}

fn orders_table_schema() -> TableSchema
{
    let mut orders_schema = TableSchema::new(ORDERS_TABLE_NAME);
    orders_schema.push_col(&Column::new(
        "order_id", 
        &LogicalColType::INT64)
    );
    orders_schema.push_col(&Column::new(
        "product_name", 
        &LogicalColType::VARCHAR)
    );
    orders_schema.push_col(&Column::new(
        "price", 
        &LogicalColType::INT64)
    );

    orders_schema
}