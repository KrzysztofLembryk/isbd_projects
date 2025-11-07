use std::iter::Enumerate;

use crate::storage::storage_structs::{CellHeader, Cell}; 

fn simple_build()
{
    let col_types = vec![false, false];
    // let storage_format = storage::StorageFormat::new(storage::MAGIC_WORD, col_types, offsets_to_rows, colrows);

    let col_data_1: Vec<i64> = vec![1, 2, 3];
    let col_data_2: Vec<i64> = vec![5, 6, 8];
}

fn create_cell(data: &Vec<i64>) -> Cell
{
   let mut cell = Cell::new(CellHeader::new()); 
   let mut cell_data_idx = 0;
   for nbr in data
   {
       let bytes = nbr.to_be_bytes();
       for byte in bytes
       {
            cell_data_idx = cell.insert_data(&bytes, cell_data_idx);
       }
       
   } 

   return cell;
}

pub fn read_csv(file_path: &str, delim: u8) 
    -> (Vec<u8>, Vec<String> , Vec<Vec<String>>)
{
    /// CSV file as first row needs to have types: 's' or 'i'
    /// as second row column names
    use std::fs::File;

    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => panic!("Error: {e}")
    };


    // TODO: check if csv:: uses streams
    let mut rdr = csv::ReaderBuilder::new()
                        .has_headers(true)
                        .delimiter(delim)
                        .from_reader(file);


    // first row needs to have types: 's' or 'i', thus they are headers
    let headers = rdr.headers().unwrap().clone();
    let types_vec = Vec::from_iter(headers.iter());
    let types_vec: Vec<u8> = types_vec.iter().map(|x| {
            if *x == "s" {1}
            else if *x == "i" {0}
            else{2} // unsuppoorted type
        })
        .collect();

    let mut columns: Vec<Vec<String>> = Vec::new();
    let mut col_names: Vec<String> = Vec::new();

    let n_cols = types_vec.len();

    // Creating 2D vector, columns[0][..] are values of first column
    for _ in 0..n_cols
    {
        columns.push(Vec::new());
    }

    let mut is_col_names = true;

    for result in rdr.records()
    {
        // col names are in first row
        if is_col_names
        {
            col_names = result
                            .unwrap()
                            .iter()
                            .map(|x| x.to_string())
                            .collect();
            is_col_names = false;
        }
        else 
        {
            let record = match result {
                Ok(r) => r,
                Err(e) => panic!("Error: for loop else branch in read_csv: {e}")
            };

            for (idx, col_val) in record.iter().enumerate()
            {
                columns
                    .get_mut(idx % n_cols)
                    .unwrap()
                    .push(String::from(col_val));
            }
        }
    }
    (types_vec, col_names, columns)
}