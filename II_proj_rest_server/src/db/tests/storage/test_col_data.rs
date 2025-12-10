use crate::db::storage::col_header::ColHeader;
use crate::db::storage::col_data::{ColData};
use crate::db::constants::LogicalColType;
use crate::db::constants::BATCH_SIZE;

use std::collections::VecDeque;
const DIR_PATH: &str = "./db_data";

#[tokio::test]
async fn test_read_from_mult_files_i64() {

    let mut batch: Vec<i64> = vec![
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10       
    ];

    save_two_files_i64(&batch).await;

    let file_path_0 = "./db_data/int_mult_files_0";
    let file_path_1 = "./db_data/int_mult_files_1";

    let mut file_paths = VecDeque::new();
    file_paths.push_back(file_path_0);
    file_paths.push_back(file_path_1);

    let col_data_read = ColData::<i64>::read_from_file(
                            file_paths, 
                            DIR_PATH
                        ).await.unwrap();

    batch.extend_from_slice(&batch.clone());
    let expected_data = batch;
    
    assert_eq!(col_data_read.n_rows(), expected_data.len());
    assert_eq!(&col_data_read.data, &expected_data);


    // Remove the file after test
    if std::fs::remove_file(file_path_0).is_err() {
        eprintln!("Warning: failed to remove {}", file_path_0);
    }

    if std::fs::remove_file(file_path_1).is_err() {
        eprintln!("Warning: failed to remove {}", file_path_1);
    }
}

#[tokio::test]
async fn test_read_from_one_file_with_negative_vals_i64() {


    let expected_data: Vec<i64> = vec![
        2137, -69, 10, 20, 20000, 88000, 0, 273, 69696900, -7
        ];

    save_one_file_i64(&expected_data).await;    
    // Get the file path that was created
    let file_path = "./db_data/int_test_col_0";
    
    // Step 2: Create VecDeque with one file path
    let mut file_paths = VecDeque::new();
    file_paths.push_back(file_path);
    
    // Step 3: Read the data back from file
    let col_data_read = ColData::<i64>::read_from_file(
                            file_paths, 
                            DIR_PATH
                        ).await.unwrap();

    let expected_data: Vec<i64> = vec![
        2137, -69, 10, 20, 20000, 88000, 0, 273, 69696900, -7
        ];


    assert_eq!(col_data_read.n_rows(), expected_data.len());
    assert_eq!(&col_data_read.data, &expected_data);

    // Remove the file after test
    if std::fs::remove_file(file_path).is_err() {
        eprintln!("Warning: failed to remove {}", file_path);
    }
}

#[tokio::test]
async fn test_read_from_file_str() {
    
    save_one_file_str().await;
    // Get the file path that was created
    let file_path = "./db_data/test_name_0";
    
    // Step 2: Create VecDeque with one file path
    let mut file_paths = VecDeque::new();
    file_paths.push_back(file_path);
    
    // Step 3: Read the data back from file
    let col_data_read = ColData::<String>::read_from_file(
                            file_paths, 
                            DIR_PATH
                        ).await.unwrap();

    let expected_data: Vec<String> = vec![
        "Alice Johnson".to_string(),
        "Bob Smith".to_string(),
        "Charlie Brown".to_string(),
        "Diana Prince".to_string(),
        "Edward Norton".to_string()
    ];

    assert_eq!(col_data_read.n_rows(), expected_data.len());
    assert_eq!(&col_data_read.data, &expected_data);

    // Remove the file after test
    if std::fs::remove_file(file_path).is_err() {
        eprintln!("Warning: failed to remove {}", file_path);
    }
}

#[tokio::test]
async fn test_read_from_mult_files_str() {

    let batch: Vec<String> = vec![
        "Alice Johnson".to_string(),
        "Bob Smith".to_string(),
        "Charlie Brown".to_string(),
        "Diana Prince".to_string(),
        "Edward Norton".to_string(),
        "Frank Miller".to_string(),
        "Grace Hopper".to_string(),
        "Henry Ford".to_string(),
        "Iris West".to_string(),
        "John Doe".to_string()
    ];

    save_two_files_str(&batch).await;

    let file_path_0 = "./db_data/str_mult_files_0";
    let file_path_1 = "./db_data/str_mult_files_1";

    let mut file_paths = VecDeque::new();
    file_paths.push_back(file_path_0);
    file_paths.push_back(file_path_1);

    let col_data_read = ColData::<String>::read_from_file(
                            file_paths, 
                            DIR_PATH
                        ).await.unwrap();

    // Create expected_data by doubling the batch
    let mut expected_data = batch.clone();
    expected_data.extend_from_slice(&batch);
    
    assert_eq!(col_data_read.n_rows(), expected_data.len());
    assert_eq!(&col_data_read.data, &expected_data);

    // Remove the files after test
    if std::fs::remove_file(file_path_0).is_err() {
        eprintln!("Warning: failed to remove {}", file_path_0);
    }

    if std::fs::remove_file(file_path_1).is_err() {
        eprintln!("Warning: failed to remove {}", file_path_1);
    }
}

async fn save_two_files_str(batch: &Vec<String>)
{
    let col_name = "str_mult_files";

    let col_file_id_0: u16 = 0;
    let col_file_id_1: u16 = 1;
    let is_overflow = false;
    let initial_size_of_data = 0;
    let col_type = LogicalColType::VARCHAR;

    let header_0 = ColHeader::new(
        col_file_id_0,
        col_type,
        is_overflow,
        initial_size_of_data,
        String::from(col_name),
        DIR_PATH,
    ).unwrap();

    let header_1 = ColHeader::new(
        col_file_id_1,
        col_type,
        is_overflow,
        initial_size_of_data,
        String::from(col_name),
        DIR_PATH,
    ).unwrap();

    let mut col_data_0: ColData::<String> = ColData::<String>::new(header_0).unwrap();
    
    for chunk in batch.chunks(BATCH_SIZE) {
        col_data_0.save_to_file(chunk).await.unwrap();
    }

    let mut col_data_1: ColData::<String> = ColData::<String>::new(header_1).unwrap();
    
    for chunk in batch.chunks(BATCH_SIZE) {
        col_data_1.save_to_file(chunk).await.unwrap();
    }
}


async fn save_two_files_i64(batch: &Vec<i64>)
{
    let col_name = "int_mult_files";

    let col_file_id_0: u16 = 0;
    let col_file_id_1: u16 = 1;
    let is_overflow = false;
    let initial_size_of_data = 0;
    let col_type = LogicalColType::INT64;


    let header_0 = ColHeader::new(
        col_file_id_0,
        col_type,
        is_overflow,
        initial_size_of_data,
        String::from(col_name),
        DIR_PATH,
    ).unwrap();

    let header_1 = ColHeader::new(
        col_file_id_1,
        col_type,
        is_overflow,
        initial_size_of_data,
        String::from(col_name),
        DIR_PATH,
    ).unwrap();


    let mut col_data: ColData::<i64> = ColData::<i64>::new(header_0).unwrap();
    
    for batch in batch.chunks(BATCH_SIZE) {
        col_data.save_to_file(batch).await.unwrap();
    }

    let mut col_data: ColData::<i64> = ColData::<i64>::new(header_1).unwrap();
    
    for batch in batch.chunks(BATCH_SIZE) {
        col_data.save_to_file(batch).await.unwrap();
    }
}

async fn save_one_file_i64(batch: &Vec<i64>) 
{
    let col_name = "int_test_col";

    let col_file_id: u16 = 0;
    let is_overflow = false;
    let initial_size_of_data = 0;
    let col_type = LogicalColType::INT64;

    let header = ColHeader::new(
        col_file_id,
        col_type,
        is_overflow,
        initial_size_of_data,
        String::from(col_name),
        DIR_PATH,
    ).unwrap();

    let mut col_data: ColData::<i64> = ColData::<i64>::new(header).unwrap();
    
    for batch in batch.chunks(BATCH_SIZE) {
        col_data.save_to_file(batch).await.unwrap();
    }
}

async fn save_one_file_str() {
    
    let batch: Vec<String> = vec![
        "Alice Johnson".to_string(),
        "Bob Smith".to_string(),
        "Charlie Brown".to_string(),
        "Diana Prince".to_string(),
        "Edward Norton".to_string()
        ];

    let col_file_id: u16 = 0;
    let is_overflow = false;
    let initial_size_of_data = 0;
    let col_type = LogicalColType::VARCHAR;

    let header = ColHeader::new(
        col_file_id,
        col_type,
        is_overflow,
        initial_size_of_data,
        String::from("test_name"),
        DIR_PATH,
    ).unwrap();

    let mut col_data: ColData::<String> = ColData::<String>::new(header).unwrap();

    for batch in batch.chunks(BATCH_SIZE) {
        col_data.save_to_file(batch).await.unwrap();
    }
}