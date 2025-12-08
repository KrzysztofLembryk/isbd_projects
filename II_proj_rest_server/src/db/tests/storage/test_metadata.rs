use self::super::{are_columns_ok, is_metadata_ok, create_dir_path, create_file_path, DbMetadata, TableMetadata, ColMetadata, TableId, DbError,
TableSchema, Column, DeleteFlag};
use super::super::super::constants::{LogicalColType};
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(test)]
mod db_metadata_struct {
    use super::*;

    #[test]
    fn test_get_tables_correct()
    {
        let db_meta = get_correct_db_metadata();
        let tables = db_meta.get_tables();
        
        assert_eq!(tables.len(), db_meta.tables_metadata.len());

        for tab in tables
        {
            let tab_id = tab.table_id();
            let tab_meta = db_meta.tables_metadata.get(&tab_id);

            assert!(tab_meta.is_some());
            assert_eq!(tab.name(), tab_meta.unwrap().table_name());
        }
    }

    #[test]
    fn test_get_tables_empty()
    {
        let table_count: u16 = 0;
        let data_dir_path = String::from("./db/");
        let metadata_file_path = String::from("./metadata");
        let table_map: HashMap<TableId, TableMetadata> = HashMap::new();

        let db_meta = DbMetadata::new(
            table_count,
            table_map,
            &metadata_file_path,
            &data_dir_path
        ).expect("Failed to create empty DbMetadata");

        let tables = db_meta.get_tables();
        
        assert_eq!(tables.len(), 0);
    }

    #[test]
    fn test_delete_table_success()
    {
        let mut db_meta = get_correct_db_metadata();
        let table_id = *db_meta.tables_metadata.keys().next().unwrap();
        
        // Mark table for deletion
        db_meta.mark_table_for_deletion(&table_id).expect("Failed to mark table for deletion");
        
        // Delete the table
        let result = db_meta.delete_table(&table_id);
        
        assert!(result.is_ok());
        let deleted_meta = result.unwrap();
        
        // Verify table was removed
        assert!(!db_meta.tables_metadata.contains_key(&table_id));
        assert!(!db_meta.tables_states.contains_key(&table_id));
        assert!(!db_meta.table_name_to_id_map.contains_key(deleted_meta.table_name()));
    }

    #[test]
    fn test_delete_table_not_found()
    {
        let mut db_meta = get_correct_db_metadata();
        let non_existent_id = Uuid::new_v4();
        
        let result = db_meta.delete_table(&non_existent_id);
        
        assert!(result.is_err());
        assert!(matches!(result, Err(DbError::NotFound(_))));
    }

    #[test]
    fn test_delete_table_not_marked_for_deletion()
    {
        let mut db_meta = get_correct_db_metadata();
        let table_id = *db_meta.tables_metadata.keys().next().unwrap();
        
        // Try to delete without marking for deletion first
        let result = db_meta.delete_table(&table_id);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_table_with_active_queries()
    {
        let mut db_meta = get_correct_db_metadata();
        let table_id = *db_meta.tables_metadata.keys().next().unwrap();
        
        // Mark table for deletion
        db_meta.mark_table_for_deletion(&table_id).expect("Failed to mark table for deletion");
        
        // Simulate active query on table
        db_meta.tables_states.get_mut(&table_id).unwrap().n_queries_operating_on_table += 1;
        
        // Try to delete with active queries
        let result = db_meta.delete_table(&table_id);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_table_preserves_other_tables()
    {
        let mut db_meta = get_correct_db_metadata();
        let initial_count = db_meta.tables_metadata.len();
        
        let table_id = *db_meta.tables_metadata.keys().next().unwrap();
        
        // Mark and delete table
        db_meta.mark_table_for_deletion(&table_id).expect("Failed to mark table for deletion");
        let result = db_meta.delete_table(&table_id);
        
        assert!(result.is_ok());
        
        // Verify other tables remain
        assert_eq!(db_meta.tables_metadata.len(), initial_count - 1);
        assert_eq!(db_meta.tables_states.len(), initial_count - 1);
    }

    #[test]
    fn test_put_table_success()
    {
        let mut db_meta = get_correct_db_metadata();
        let initial_count = db_meta.table_count;
        
        let mut table_schema = TableSchema::new("new_table");
        table_schema.push_col(&Column::new("id", &LogicalColType::INT64));
        table_schema.push_col(&Column::new("name", &LogicalColType::VARCHAR));
        
        let result = db_meta.put_table(&table_schema);
        
        assert!(result.is_ok());
        let table_id = result.unwrap();
        
        // Verify table was added
        assert!(db_meta.tables_metadata.contains_key(&table_id));
        assert!(db_meta.tables_states.contains_key(&table_id));
        assert_eq!(db_meta.table_name_to_id_map.get("new_table"), Some(&table_id));
        assert_eq!(db_meta.table_count, initial_count + 1);
        assert_eq!(db_meta.nbr_of_metadata_changes, 1);
    }

    #[test]
    fn test_put_table_increments_metadata_changes()
    {
        let mut db_meta = get_correct_db_metadata();
        
        let mut table_schema_1 = TableSchema::new("table_a");
        table_schema_1.push_col(&Column::new("col1", &LogicalColType::INT64));
        
        let mut table_schema_2 = TableSchema::new("table_b");
        table_schema_2.push_col(&Column::new("col1", &LogicalColType::INT64));
        
        db_meta.put_table(&table_schema_1).expect("Failed to add first table");
        db_meta.put_table(&table_schema_2).expect("Failed to add second table");
        
        assert_eq!(db_meta.nbr_of_metadata_changes, 2);
    }

    #[test]
    fn test_put_table_creates_column_filepaths()
    {
        let mut db_meta = get_correct_db_metadata();
        
        let mut table_schema = TableSchema::new("test_table");
        table_schema.push_col(&Column::new("col_a", &LogicalColType::INT64));
        table_schema.push_col(&Column::new("col_b", &LogicalColType::VARCHAR));
        
        let result = db_meta.put_table(&table_schema);
        
        assert!(result.is_ok());
        let table_id = result.unwrap();
        
        let table_meta = db_meta.tables_metadata.get(&table_id).unwrap();
        
        // Each column should have one file path
        assert_eq!(table_meta.columns.len(), 2);
        for col in &table_meta.columns {
            assert_eq!(col.c_files.len(), 1);
            assert!(col.c_files[0].contains(&table_id.to_string()));
            assert!(col.c_files[0].contains("test_table"));
            assert!(col.c_files[0].contains(&col.c_name));
        }
    }

    #[test]
    fn test_put_table_non_ascii_column_name()
    {
        let mut db_meta = get_correct_db_metadata();
        
        let mut table_schema = TableSchema::new("bad_table");
        table_schema.push_col(&Column::new("col_ąćę", &LogicalColType::INT64));
        
        let result = db_meta.put_table(&table_schema);
        
        assert!(result.is_err());
        assert!(matches!(result, Err(DbError::InvalidColumnName { .. })));
    }

    #[test]
    fn test_put_table_duplicate_column_names()
    {
        let mut db_meta = get_correct_db_metadata();
        
        let mut table_schema = TableSchema::new("dup_table");
        table_schema.push_col(&Column::new("col1", &LogicalColType::INT64));
        table_schema.push_col(&Column::new("col1", &LogicalColType::VARCHAR)); // Duplicate
        
        let result = db_meta.put_table(&table_schema);
        
        assert!(result.is_err());
        assert!(matches!(result, Err(DbError::InvalidColumnName { .. })));
    }

    #[test]
    fn test_put_table_empty_columns()
    {
        let mut db_meta = get_correct_db_metadata();
        
        let table_schema = TableSchema::new("empty_table");
        
        let result = db_meta.put_table(&table_schema);
        
        assert!(result.is_ok());
        let table_id = result.unwrap();
        
        let table_meta = db_meta.tables_metadata.get(&table_id).unwrap();
        assert_eq!(table_meta.columns.len(), 0);
    }

    #[test]
    fn test_put_table_initializes_table_state()
    {
        let mut db_meta = get_correct_db_metadata();
        
        let mut table_schema = TableSchema::new("state_table");
        table_schema.push_col(&Column::new("col1", &LogicalColType::INT64));
        
        let result = db_meta.put_table(&table_schema);
        
        assert!(result.is_ok());
        let table_id = result.unwrap();
        
        let table_state = db_meta.tables_states.get(&table_id).unwrap();
        assert_eq!(table_state.delete_flag, DeleteFlag::NoDelete);
        assert_eq!(table_state.n_queries_operating_on_table, 0);
    }

    #[test]
    fn test_put_table_preserves_existing_tables()
    {
        let mut db_meta = get_correct_db_metadata();
        let initial_tables: Vec<_> = db_meta.tables_metadata.keys().cloned().collect();
        
        let mut table_schema = TableSchema::new("new_table");
        table_schema.push_col(&Column::new("col1", &LogicalColType::INT64));
        
        db_meta.put_table(&table_schema).expect("Failed to add table");
        
        // All initial tables should still exist
        for table_id in initial_tables {
            assert!(db_meta.tables_metadata.contains_key(&table_id));
        }
    }

}

#[cfg(test)]
mod is_metadata_ok {
    use super::*;

    #[test]
    fn test_correct()
    {

        let (table_count, 
            data_dir_path,
            metadata_file_path,
            table_map
            ) = get_correct_metadata();

        assert!(is_metadata_ok(
            table_count, 
            &table_map, 
            &metadata_file_path, 
            &data_dir_path).is_ok()
        )

    }

    #[test]
    fn test_invalid_metadata_file_path()
    {
        let (table_count, 
            data_dir_path,
            _,
            table_map
        ) = get_correct_metadata();

        let invalid_metadata_path = String::from("invalid/path/../with/dots");

        assert!(is_metadata_ok(
            table_count, 
            &table_map, 
            &invalid_metadata_path, 
            &data_dir_path).is_err()
        );
    }

    #[test]
    fn test_invalid_data_dir_path()
    {
        let (table_count, 
            _,
            metadata_file_path,
            table_map
        ) = get_correct_metadata();

        let invalid_data_dir_double_dot_start = String::from("../invalid/dir");
        let invalid_data_dir_double_dot_mid = String::from("./invalid/../dir");
        // let invalid_data_dir_dot_mid = String::from("./invalid/./dir");

        assert!(is_metadata_ok(
            table_count, 
            &table_map, 
            &metadata_file_path, 
            &invalid_data_dir_double_dot_start).is_err()
        );
        assert!(is_metadata_ok(
            table_count, 
            &table_map, 
            &metadata_file_path, 
            &invalid_data_dir_double_dot_mid).is_err()
        );
    }

    #[test]
    fn test_table_count_mismatch()
    {
        let (_, 
            data_dir_path,
            metadata_file_path,
            table_map
        ) = get_correct_metadata();

        let wrong_table_count: u16 = 5;

        assert!(is_metadata_ok(
            wrong_table_count, 
            &table_map, 
            &metadata_file_path, 
            &data_dir_path).is_err()
        );
    }

    #[test]
    fn test_invalid_column_names()
    {
        let (_, 
            data_dir_path,
            metadata_file_path,
            _
        ) = get_correct_metadata();

        let mut table_map: HashMap<TableId, TableMetadata> = HashMap::new();
        let table_id = Uuid::new_v4();
        
        let columns = vec![
            ColMetadata::new("valid_col", LogicalColType::INT64),
            ColMetadata::new("invalid_ąćół", LogicalColType::VARCHAR), // Non-ASCII
        ];

        table_map.insert(
            table_id, 
            TableMetadata::new("table_invalid_cols", &table_id, columns, "")
        );

        assert!(is_metadata_ok(
            1, 
            &table_map, 
            &metadata_file_path, 
            &data_dir_path).is_err()
        );
    }

    #[test]
    fn test_column_name_too_long()
    {
        let (_, 
            data_dir_path,
            metadata_file_path,
            _
        ) = get_correct_metadata();

        let mut table_map: HashMap<TableId, TableMetadata> = HashMap::new();
        let table_id = Uuid::new_v4();
        
        let columns = vec![
            ColMetadata::new(&"a".repeat(269), LogicalColType::INT64), // Too long
        ];

        table_map.insert(
            table_id, 
            TableMetadata::new("table_long_col", &table_id, columns, "")
        );

        assert!(is_metadata_ok(
            1, 
            &table_map, 
            &metadata_file_path, 
            &data_dir_path).is_err()
        );
    }

    #[test]
    fn test_empty_tables()
    {
        let table_count: u16 = 0;
        let data_dir_path = String::from("./db/");
        let metadata_file_path = String::from("./metadata");
        let table_map: HashMap<TableId, TableMetadata> = HashMap::new();

        assert!(is_metadata_ok(
            table_count, 
            &table_map, 
            &metadata_file_path, 
            &data_dir_path).is_ok()
        );
    }
}

#[cfg(test)]
mod are_columns_ok{
    use super::*;
    
    #[test]
    fn test_correct()
    {
        let columns = get_correct_col_metadata_vec();
        let table_name = String::from("table");
    
        assert!(are_columns_ok(&columns, &table_name).is_ok());
    }
    
    #[test]
    fn test_col_name_exceeds_max_len()
    {
        let mut columns: Vec<ColMetadata> = Vec::new();
    
        let table_name = String::from("table");
        let col_name_1 = String::from("a".repeat(269));
        let col_name_2 = String::from("col_2");
        let mut col_1 = ColMetadata::new(&col_name_1, LogicalColType::INT64);
        let mut col_2 = ColMetadata::new(&col_name_2, LogicalColType::INT64);
    
        col_1.c_files.push(String::from(format!("./db/{}/{}", table_name, col_name_1)));
        col_2.c_files.push(String::from(format!("./db/{}/{}", table_name, col_name_2)));
    
        columns.push(col_1);
        columns.push(col_2);
    
        assert!(are_columns_ok(&columns, &table_name).is_err());
    }
    
    #[test]
    fn test_non_ascii()
    {
        let mut columns: Vec<ColMetadata> = Vec::new();
    
        let table_name = String::from("table");
        let col_name_1 = String::from("colą_łan");
        let col_name_2 = String::from("col_2");
        let mut col_1 = ColMetadata::new(&col_name_1, LogicalColType::INT64);
        let mut col_2 = ColMetadata::new(&col_name_2, LogicalColType::INT64);
    
        col_1.c_files.push(String::from(format!("./db/{}/{}", table_name, col_name_1)));
        col_2.c_files.push(String::from(format!("./db/{}/{}", table_name, col_name_2)));
    
        columns.push(col_1);
        columns.push(col_2);
    
        assert!(are_columns_ok(&columns, &table_name).is_err());
    }

}

#[cfg(test)]
mod create_paths {
    use super::*;

    #[test]
    fn test_correct_file_path() 
    {
        let db_dir = "./db_data";
        let table_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let table_name = "users";
        let col_name = "username";
        let idx = 0;
    
        let result = create_file_path(db_dir, &table_id, table_name, col_name, idx);
        let expected = "./db_data/550e8400-e29b-41d4-a716-446655440000_users/username_0";
    
        assert_eq!(result, expected);
    }
    
    #[test]
    fn test_correct_dir_path() 
    {
        let db_dir = "./db_data";
        let table_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let table_name = "users";
    
        let result = create_dir_path(db_dir, &table_id, table_name);
        let expected = "./db_data/550e8400-e29b-41d4-a716-446655440000_users";
    
        assert_eq!(result, expected);
    }

}

// ############################################################################
// ########################### Helper Functions ###############################
// ############################################################################

fn get_correct_db_metadata() -> DbMetadata
{
    let (table_count, data_dir_path, metadata_file_path, table_map) = get_correct_metadata();

    DbMetadata::new(
        table_count,
        table_map,
        &metadata_file_path,
        &data_dir_path
    ).expect("Failed to create correct DbMetadata in test")
}


fn get_correct_metadata() -> (u16, String, String, HashMap<TableId, TableMetadata>)
{
    let table_count: u16 = 2;
    let data_dir_path = String::from("./db/");
    let metadata_file_path = String::from("./metadata");
    let mut table_map: HashMap<TableId, TableMetadata> = HashMap::new();
    let columns = get_correct_col_metadata_vec();

    let id_1 = Uuid::new_v4();
    let id_2 = Uuid::new_v4();

    table_map.insert(id_1, TableMetadata { 
        table_name: String::from("table_1"), 
        table_id: Uuid::new_v4(),
        columns: columns.clone(),
        table_dir_path: "".to_string()
    });

    table_map.insert(id_2, TableMetadata { 
        table_name: String::from("table_2"), 
        table_id: Uuid::new_v4(),
        columns: columns,
        table_dir_path: "".to_string()
    });
    (table_count, data_dir_path, metadata_file_path, table_map)
}

fn get_correct_col_metadata_vec() -> Vec<ColMetadata>
{
    let mut columns: Vec<ColMetadata> = Vec::new();

    let table_name = String::from("table");
    let col_name_1 = String::from("col_1");
    let col_name_2 = String::from("col_2");
    let mut col_1 = ColMetadata::new(&col_name_1, LogicalColType::INT64);
    let mut col_2 = ColMetadata::new(&col_name_2, LogicalColType::INT64);

    col_1.c_files.push(String::from(format!("./db/{}/{}", table_name, col_name_1)));
    col_2.c_files.push(String::from(format!("./db/{}/{}", table_name, col_name_2)));

    columns.push(col_1);
    columns.push(col_2);

    columns
}
