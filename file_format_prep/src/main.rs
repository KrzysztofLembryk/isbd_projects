// use std::collections::{self, HashMap};
use std::fs::File;
use std::io::{Read, Write};
// use std::fs::OpenOptions;

use std::vec;

// use file_format_prep::csv_reader;
// use file_format_prep::storage::metadata_structs as meta_structs;
// use file_format_prep::constants::METADATA_FILE_PATH;
// use file_format_prep::storage::column_structs::{ColHeader};
use file_format_prep::db_manager::DbManager;
use file_format_prep::storage::col_data::ColData;
use file_format_prep::storage::col_header::ColHeader;
use file_format_prep::constants::{AllowedColTypes, DB_DATA_DIR};

fn main() 
{
    let delim = b'\t';
    let tsv_file_path = "./db_data/sample_med.tsv";

    let mut db_manager = DbManager::new(DB_DATA_DIR);

    // match db_manager.init_from_csv(tsv_file_path, delim)
    // {
    //     Ok(_) => (),
    //     Err(e) => panic!("{e}")
    // }

    db_manager.init_db().unwrap();
    db_manager.read_col_data("int1").unwrap();

}
