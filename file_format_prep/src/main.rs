use std::collections;

use file_format_prep::csv_reader;
use file_format_prep::storage::metadata_structs as meta_structs;
use file_format_prep::constants::METADATA_FILE_PATH;
use file_format_prep::storage::column_structs::{ColHeader};

fn main() {
    // let (types, names, _columns) = s_builder::read_csv("./db_data/sample.tsv", b'\t');
    // let (types, names, _) = csv_reader::read_csv("./db_data/sample_med.tsv", b'\t');

    // let metadata = match meta_structs::DbMetadata::new_basic(types, names)
    //             {
    //                 Ok(m) => m,
    //                 Err(e) => panic!("{e}") 
    //             };
    
    // let metadata = match meta_structs::DbMetadata::new_basic(Vec::new(), Vec::new())
    //             {
    //                 Ok(m) => m,
    //                 Err(e) => panic!("{e}") 
    //             };
    // println!("Making metadata success!");

    // match metadata.save_to_file("./db_metadata_empty")
    // {
    //     Ok(_) => (),
    //     Err(e) => panic!("{e}")
    // }

    // let metadata = meta_structs::DbMetadata::read_from_file(METADATA_FILE_PATH).unwrap();

    // println!("METADATA READ SUCCESS:");
    // println!("{}", metadata);

    let col_id = 0;
    let col_type = 1;
    let is_overflow = false;
    let size_of_data = 25;
    let col_name = String::from("my_first_column");

    let col_h = ColHeader::new(
        col_id, 
        col_type, 
        is_overflow, 
        size_of_data, 
        col_name).unwrap();

    

}

