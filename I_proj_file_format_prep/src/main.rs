use file_format_prep::db_manager::DbManager;
use file_format_prep::constants::{DB_DATA_DIR};

fn main() 
{
    let delim = b'\t';
    let tsv_file_path = "./db_data/sample_med.tsv";

    let mut db_manager = DbManager::new(DB_DATA_DIR);

    db_manager.init_from_csv(tsv_file_path, delim).unwrap();

    db_manager.init_db().unwrap();
    db_manager.read_all_col_data().unwrap();
}
