use crate::db::storage::col_header::ColHeader;
use crate::db::storage::col_data::{ColData};
use crate::db::constants::LogicalColType;

use std::collections::VecDeque;

#[tokio::test]
async fn test_save_to_file() {
    
    let dir_path = "./db_data";
    // let file_path = "./db_data/name_0";

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
        dir_path,
    ).unwrap();

    let mut col_data: ColData::<String> = ColData::<String>::new(header).unwrap();
    
    col_data.save_to_file(&batch[..3]).await.unwrap();
    col_data.save_to_file(&batch[3..=4]).await.unwrap();
}

#[tokio::test]
async fn test_read_from_file() {
    
    // Get the file path that was created
    let dir_path = "./db_data";
    let file_path = "./db_data/test_name_0";
    
    // Step 2: Create VecDeque with one file path
    let mut file_paths = VecDeque::new();
    file_paths.push_back(file_path);
    
    // Step 3: Read the data back from file
    let col_data_read = ColData::<String>::read_from_file(
                            file_paths, 
                            dir_path
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
    println!("read data: {:?}", col_data_read);

    // Remove the file after test
    if std::fs::remove_file(file_path).is_err() {
        eprintln!("Warning: failed to remove {}", file_path);
    }
}

