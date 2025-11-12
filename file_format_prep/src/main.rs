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
use file_format_prep::storage::column_structs::{ColData, ColHeader};
use file_format_prep::constants::AllowedColTypes;

fn main() 
{
    // FLOW
    // - we create db_manager
    // - we read metadata file and in db_manager create metadata obj
    // - we ask user to choose columns' names to calculated stuff
    // - for given col_name we create ColData obj
    // - ColData obj takes care of reading all data from file and calculating
    //   mean or sth

    // println!("main db_manager new");
    // let mut db_manager = DbManager::new();
    // println!("init csv");
    // match db_manager.init_from_csv("./db_data/sample_med.tsv")
    // {
    //     Ok(_) => (),
    //     Err(e) => panic!("{e}")
    // }
    // db_manager.start_db().unwrap();

    let mut header = ColHeader::new_empty(AllowedColTypes::IntType, String::from("int1")).unwrap();

    let mut col_data: ColData<i64> = ColData::new(header).unwrap();

    // let vals: Vec<i64> = vec![1, 4, -5, 0, 10, 8, -2, -4, 0, -5];
    let vals: Vec<i64> = vec![1, 2, 3, 4, 5, 5, -5, -5, 0, 10];
    // let vals: Vec<i64> = vec![1, 50];

    for val in vals
    {
        col_data.push(val).unwrap();
    }

    let (file_path, _) = col_data.create_and_save_to_file();

    let f = File::open(file_path).unwrap();
    let _ = ColData::read_from_file(f);

    // let read_data = new_col.data();

    // println!("Data after read and decoding ");
    // println!("{:?}", read_data);

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

