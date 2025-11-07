use file_format_prep::storage::storage_builder as s_builder;
use file_format_prep::storage::metadata_structs as meta_structs;

fn main() {
    let (types, names, _columns) = s_builder::read_csv("./db_data/sample.tsv", b'\t');

    // let metadata = match meta_structs::DbMetadata::new_basic(types, names)
    //             {
    //                 Ok(m) => m,
    //                 Err(e) => panic!("{e}") 
    //             };
    
    // println!("Making metadata success!");

    // match metadata.save_to_file("./db_metadata")
    // {
    //     Ok(_) => (),
    //     Err(e) => panic!("{e}")
    // }
    let metadata = meta_structs::DbMetadata::read_from_file(meta_structs::METADATA_FILE_PATH).unwrap();

    println!("METADATA READ SUCCESS:");
    println!("{}", metadata);
}

