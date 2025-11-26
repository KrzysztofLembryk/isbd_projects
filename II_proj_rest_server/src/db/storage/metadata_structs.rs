use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::vec;
use regex::Regex;
use serde;
use serde_json;

use crate::db::errors::{DbError};
use crate::db::constants::{DB_DATA_DIR, MAX_COL_NAME_LEN, MAX_COL_COUNT, AllowedColType, FILE_PATH_REGEX};
use crate::schemas::table;
use uuid::Uuid;

type TableName = String;
type TableMetadata = (TableName, HashMap<ColumnName, ColMetadata>);
type ColumnName = String;
type FilePath = String;
type TableId = Uuid;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ColMetadata
{
    // table_name: TableName, // ???
    c_name: ColumnName,
    c_type: AllowedColType,
    c_files: Vec<FilePath>
}

impl ColMetadata
{
    pub fn new(c_name: &str, c_type: AllowedColType) -> ColMetadata
    {
        ColMetadata { 
            c_name: String::from(c_name), 
            c_type, 
            c_files: Vec::new()
        }
    }

}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DbMetadata
{
    /// In DbMetadata struct we will store all file names, dirs, etc 
    /// Each table has its own directory: 'table_name/'
    /// Thus we cannot have two tables with the same name.
    /// -- table_count: variable that remembers how many tables in db we have
    /// -- table_cols: hash map in which for given table_name we store all its
    ///                columns metadata
    table_count: u16,
    table_map: HashMap<TableId, TableMetadata>,
    #[serde(skip)]
    file_path: String
}

impl DbMetadata  
{
    pub fn new(
        table_count: u16,
        mut table_map: HashMap<TableId, TableMetadata>,
        file_path: &str
    ) -> Result<DbMetadata, DbError>
    {
        check_metadata_correctness(table_count, &table_map, file_path)?;
        // For each column we create a filepath: DB_DIR/table_name/col_name_0
        create_filepaths_for_all_columns(&mut table_map)?;

        Ok(DbMetadata {
            table_count: table_count,
            table_map,
            file_path: String::from(file_path)
        })
    }

    pub fn new_empty(file_path: &str) -> Result<DbMetadata, DbError>
    {
        let table_count = 0;
        let table_cols = HashMap::new();
        DbMetadata::new(table_count, table_cols, file_path)
    }

    /// We do not encode metadata when saving to file
    pub fn save_to_file(&self) -> Result<(), DbError>
    {
        let mut f = File::create(self.file_path.clone())?;
        let buf = serde_json::to_vec(self)
                            .or_else(|e| 
                                return Err(DbError::Other(e.to_string()))
                            )?;

        f.write_all(&buf[..])?;
        f.flush()?;

        Ok(())
    }
    
    pub fn read_from_file(path: &str) -> Result<DbMetadata, DbError>
    {
        let mut f = File::open(path)?;
        let mut buf = vec![]; 

        f.read_to_end(&mut buf)?;

        let mut metadata: DbMetadata = serde_json::from_slice(&buf[..])
                                .or_else(|e| 
                                    return Err(DbError::IoError(e.into()))
                                )?;
        metadata.file_path = String::from(path);

        check_metadata_correctness(
            metadata.table_count, 
            &metadata.table_map, 
            &metadata.file_path)?;

        Ok(metadata)
    }

    pub fn append_new_file_paths(
        &mut self, 
        table_name: &str,
        col_name: &str, 
        file_path: &str
    ) -> Result<(), DbError>
    {
        if self.col_files_paths.contains_key(col_name)
        {
            // if we have such col name in map we just pushback a new file path
            self.col_files_paths.get_mut(col_name).unwrap().push(file_path);

            // and also update variable storing number of cols for given column
            let idx = self.col_names_idxs.get(col_name).unwrap();

            let file_count = self.col_files_count.get_mut(*idx).unwrap();
            *file_count += 1;
        }
        else 
        {
            return Err(DbError::Other(
                format!("DbMetadata::append_new_file_path: col_name: {} is not present in db_metadata", col_name)));
        }

        Ok(())
    }

    // ###################################################################### 
    // ############################ GETTERS #################################
    // ###################################################################### 

}

//##############################################################################
//######################### PRIVATE HELPER FUNCTIONS ###########################
//##############################################################################

fn check_metadata_correctness(
        table_count: u16,
        table_map: &HashMap<TableId, TableMetadata>,
        file_path: &str
) -> Result<(), DbError>
{
    let re = Regex::new(FILE_PATH_REGEX).unwrap();
    if !re.is_match(&file_path) 
    {
        return Err(DbError::Other(format!("DbMetadata::new: file_path: '{}', does not satisfy regex: '{}'", file_path, FILE_PATH_REGEX)));
    }

    if table_count as usize != table_map.len()
    {
        return Err(DbError::SizeMismatch{
            msg: format!("DbMetadata::new: table_count has diff len than table_cols map"),
            size_1: table_count as usize,
            size_2: table_map.len()
        });
    }

    for (_, table_meta) in table_map
    {
        let (table_name, col_map) = table_meta;
        if col_map.len() > MAX_COL_COUNT
        {
            return Err(DbError::SizeExceeded{
                msg: format!("DbMetadata::new: number of columns in table: '{}' is greater than MAX_COL_COUNT", table_name),
                max: MAX_COL_COUNT
            });
        }

        for (col_name, _) in col_map
        {
            if col_name.len() > MAX_COL_NAME_LEN
            {
                return Err(DbError::SizeExceeded{
                    msg: format!("DbMetadata::new: column: '{}' length exceeds MAX_COL_NAME_LEN ", col_name),
                    max: MAX_COL_NAME_LEN
                });
            }
        }
    }

    Ok(())
}

fn create_filepaths_for_all_columns(
        table_map: &mut HashMap<TableId, TableMetadata>,
) -> Result<(), DbError>
{
    for (_, table_meta) in table_map
    {
        let (table_name, col_map) = table_meta;

        for (name, col_meta) in col_map
        {
            if *name != col_meta.c_name
            {
                return Err(DbError::Other(format!("DbMetadata::new:: name: '{}' of column in hashmap KEY, is not the same as name: '{}' of column in hashmap VALUE (ColMetadata) in table: '{}'", name, col_meta.c_name, table_name)));
            }

            // At first we have only one file for each column, when we read
            // enough data, we will create another one if the first is full
            let file_path = format!("{DB_DATA_DIR}/{table_name}/{name}_0");

            col_meta.c_files.push(file_path);
        }
    }

    Ok(())
}

