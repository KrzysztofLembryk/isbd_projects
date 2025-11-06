use file_format_prep::storage::storage_builder as s_builder;

fn main() {
    let (types_names_vec, columns) = s_builder::read_csv("./db_data/sample.tsv", b'\t');


}

