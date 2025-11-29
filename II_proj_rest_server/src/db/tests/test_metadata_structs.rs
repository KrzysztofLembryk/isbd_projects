use self::super::*;

// ############################################################################
// #################### tests for: is_metadata_ok ############################
// ############################################################################
#[test]
fn test_is_metadata_ok_correct()
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

fn get_correct_metadata() -> (u16, String, String, HashMap<TableId, TableMetadata>)
{
    let table_count: u16 = 2;
    let data_dir_path = String::from("./db/");
    let metadata_file_path = String::from("./metadata");
    let mut table_map: HashMap<TableId, TableMetadata> = HashMap::new();
    let columns = get_correct_col_metadata_map();

    let id_1 = Uuid::new_v4();
    let id_2 = Uuid::new_v4();

    table_map.insert(id_1, TableMetadata { 
        table_name: String::from("table_1"), 
        columns: columns.clone()
    });

    table_map.insert(id_2, TableMetadata { 
        table_name: String::from("table_2"), 
        columns: columns 
    });
    (table_count, data_dir_path, metadata_file_path, table_map)
}

// ############################################################################
// ##################### tests for: are_columns_ok ############################
// ############################################################################
#[test]
fn test_are_columns_ok_correct()
{
    let columns = get_correct_col_metadata_map();
    let table_name = String::from("table");

    assert!(are_columns_ok(&columns, &table_name).is_ok());
}

#[test]
fn test_are_columns_ok_col_name_key_different_from_col_metadata()
{
    let mut columns: HashMap<ColumnName, ColMetadata> = HashMap::new();

    let table_name = String::from("table");
    let col_name_1 = String::from("col_1");
    let col_name_2 = String::from("col_2");
    let mut col_1 = ColMetadata::new(&col_name_1, LogicalColType::INT64);
    let mut col_2 = ColMetadata::new(&col_name_2, LogicalColType::INT64);

    col_1.c_files.push(String::from(format!("./db/{}/{}", table_name, col_name_1)));
    col_2.c_files.push(String::from(format!("./db/{}/{}", table_name, col_name_2)));

    columns.insert(String::from("col_1_different"), col_1);
    columns.insert(col_name_2, col_2);

    assert!(are_columns_ok(&columns, &table_name).is_err());
}

#[test]
fn test_are_columns_ok_col_name_exceeds_max_len()
{
    let mut columns: HashMap<ColumnName, ColMetadata> = HashMap::new();

    let table_name = String::from("table");
    let col_name_1 = String::from("a".repeat(269));
    let col_name_2 = String::from("col_2");
    let mut col_1 = ColMetadata::new(&col_name_1, LogicalColType::INT64);
    let mut col_2 = ColMetadata::new(&col_name_2, LogicalColType::INT64);

    col_1.c_files.push(String::from(format!("./db/{}/{}", table_name, col_name_1)));
    col_2.c_files.push(String::from(format!("./db/{}/{}", table_name, col_name_2)));

    columns.insert(col_name_1, col_1);
    columns.insert(col_name_2, col_2);

    assert!(are_columns_ok(&columns, &table_name).is_err());
}

#[test]
fn test_are_columns_ok_non_ascii()
{
    let mut columns: HashMap<ColumnName, ColMetadata> = HashMap::new();

    let table_name = String::from("table");
    let col_name_1 = String::from("colą_łan");
    let col_name_2 = String::from("col_2");
    let mut col_1 = ColMetadata::new(&col_name_1, LogicalColType::INT64);
    let mut col_2 = ColMetadata::new(&col_name_2, LogicalColType::INT64);

    col_1.c_files.push(String::from(format!("./db/{}/{}", table_name, col_name_1)));
    col_2.c_files.push(String::from(format!("./db/{}/{}", table_name, col_name_2)));

    columns.insert(col_name_1, col_1);
    columns.insert(col_name_2, col_2);

    assert!(are_columns_ok(&columns, &table_name).is_err());
}

fn get_correct_col_metadata_map() -> HashMap<ColumnName, ColMetadata>
{
    let mut columns: HashMap<ColumnName, ColMetadata> = HashMap::new();

    let table_name = String::from("table");
    let col_name_1 = String::from("col_1");
    let col_name_2 = String::from("col_2");
    let mut col_1 = ColMetadata::new(&col_name_1, LogicalColType::INT64);
    let mut col_2 = ColMetadata::new(&col_name_2, LogicalColType::INT64);

    col_1.c_files.push(String::from(format!("./db/{}/{}", table_name, col_name_1)));
    col_2.c_files.push(String::from(format!("./db/{}/{}", table_name, col_name_2)));

    columns.insert(col_name_1, col_1);
    columns.insert(col_name_2, col_2);

    columns
}

// ############################################################################
// ###################### tests for: create paths #############################
// ############################################################################
#[test]
fn test_create_file_path() 
{
    // TODO:
    let db_dir = "./db";
    // let id = Uuid::n
    // assert!(check_col_name_correctness(&too_long).is_err());
}

#[test]
fn test_create_dir_path() 
{
    // TODO:
    let db_dir = "./db";
    // let id = Uuid::n
    // assert!(check_col_name_correctness(&too_long).is_err());
}