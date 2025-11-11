// use std::collections::{self, HashMap};
// use std::fs::File;
// use std::io::{Read, Write};
// use std::fs::OpenOptions;

// use file_format_prep::csv_reader;
// use file_format_prep::storage::metadata_structs as meta_structs;
// use file_format_prep::constants::METADATA_FILE_PATH;
// use file_format_prep::storage::column_structs::{ColHeader};
use file_format_prep::db_manager::DbManager;

fn main() {

    let mut db_manager = DbManager::new();
    db_manager.init_from_csv("./db_data/sample_med.tsv");
    // db_manager.start_db().unwrap();

}

// fn col_header_test()
// {

//     let col_id = 0;
//     let col_type = 1;
//     let is_overflow = false;
//     let size_of_data = 25;
//     let col_name = String::from("my_first_column");

//     // let col_h = ColHeader::new(
//     //     col_id, 
//     //     col_type, 
//     //     is_overflow, 
//     //     size_of_data, 
//     //     col_name).unwrap();
//     // let (file_name, mut f_handler) = col_h.save_to_file().unwrap();

//     // println!("created file_name is {}", file_name);
    
//     // println!("We will write sth more to the end of create file:");
//     // // f_handler.write("ala ma kota xdxdx".as_bytes()).unwrap();
//     // drop(f_handler);

//     // let mut f = File::open(&file_name).unwrap();
//     let file_name = String::from("./db_data/my_first_column_0");

//     // in order to modify already existing data we need to specify
//     // both read and write to true
//     let mut f = OpenOptions::new()
//                 .write(true)
//                 .read(true)
//                 .open(&file_name)
//                 .unwrap(); 

//     let mut buf: [u8; 255] = [0; 255];

//     let bytes_read = f.read(&mut buf).unwrap();

//     let mut buff_idx = 0;
//     let mut col_h = ColHeader::read_from_buf(&mut buff_idx, bytes_read, &buf).unwrap();

//     println!("colHeader: {}", col_h);

//     match col_h.increase_data_size(44)
//     {
//         Ok(_) => (),
//         Err(e) => {
//             println!("{e}");
//         }
//     };

//     println!("modifying data in file");
//     col_h.modify_data_size_in_file(&mut f).unwrap();
// }

