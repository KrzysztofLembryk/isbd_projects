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
