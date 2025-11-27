use std::collections::HashMap;
use std::fs::{self, File};
use std::path::Path;
use std::io::{Read};
use std::vec;
use regex::Regex;
use serde;
use serde_json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;
use tokio::fs as t_fs;

use crate::db::errors::{DbError};
use crate::db::constants::{DB_DATA_DIR, MAX_COL_NAME_LEN, MAX_COL_COUNT, LogicalColType, FILE_PATH_REGEX};
use crate::schemas::table::{TableSchema, ShallowTable};
use crate::schemas::column::{Column};

type TableName = String;
type ColumnName = String;
type FilePath = String;
type TableId = Uuid;

// ####################################################
// DBMETADATA SHOULD BE CRITICAL SECTION - i.e. RwLock?
// ####################################################
#[derive(serde::Serialize, serde::Deserialize, Clone)]
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
    metadata_file_path: String
}

impl DbMetadata  
{
    pub fn new(
        table_count: u16,
        mut table_map: HashMap<TableId, TableMetadata>,
        metadata_file_path: &str
    ) -> Result<DbMetadata, DbError>
    {
        check_metadata_correctness(table_count, &table_map, metadata_file_path)?;
        // For each column we create a filepath: DB_DIR/table_name/col_name_0
        create_filepaths_for_all_columns(&mut table_map)?;

        Ok(DbMetadata {
            table_count: table_count,
            table_map,
            metadata_file_path: String::from(metadata_file_path)
        })
    }

    pub fn new_empty(metadata_file_path: &str) -> Result<DbMetadata, DbError>
    {
        let table_count = 0;
        let table_map = HashMap::new();
        DbMetadata::new(table_count, table_map, metadata_file_path)
    }

    /// We do not encode metadata when saving to file.
    /// When saving we overwrite whole previous metadata file
    /// Maybe we should change save_to_file to get COPY of DbMetadata so that
    /// we will not block mutex while saving to file????
    pub async fn save_to_file(metadata: DbMetadata) -> Result<(), DbError>
    {
        let mut f = t_fs::File::create(metadata.metadata_file_path.clone()).await?;
        let buf = serde_json::to_vec(&metadata)
                            .or_else(|e| 
                                return Err(DbError::Other(e.to_string()))
                            )?;

        f.write_all(&buf[..]).await?;
        f.flush().await?;

        Ok(())
    }
    
    pub async fn read_from_file(path: &str) -> Result<DbMetadata, DbError>
    {
        let mut f = t_fs::File::open(path).await?;
        let mut buf = vec![]; 

        f.read_to_end(&mut buf).await?;

        let mut metadata: DbMetadata = serde_json::from_slice(&buf[..])
                                .or_else(|e| 
                                    return Err(DbError::IoError(e.into()))
                                )?;
        metadata.metadata_file_path = String::from(path);

        check_metadata_correctness(
            metadata.table_count, 
            &metadata.table_map, 
            &metadata.metadata_file_path)?;

        Ok(metadata)
    }

    pub async fn get_tables(&self) -> Vec<ShallowTable>
    {
        let mut tables: Vec<ShallowTable> = Vec::new();

        for (t_id, t_meta) in &self.table_map
        {
            tables.push(ShallowTable::new(t_id, &t_meta.table_name));
        }

        tables
    }

    pub async fn get_table_details(
        &self, 
        table_id: &Uuid
    ) -> Result<TableSchema, DbError>
    {
        if let Some(tab_meta) = self.table_map.get(table_id)
        {
            return Ok(tab_meta.into_table_schema());
        }

        Err(DbError::NotFound(format!("Table with id: {}, not found in db.", table_id)))
    }

    /// Function receives TableSchema and adds table with its columns to 
    /// metadata structure, it also **creates dirs and files** for this table
    pub async  fn put_table(
        &mut self, 
        table_schema: &TableSchema
    ) -> Result<TableId, DbError>
    {
        let table_id = TableId::new_v4();
        let table_name = table_schema.name();
        let columns = table_schema.columns();
        let file_idx = 0;
        let mut col_map: HashMap<ColumnName, ColMetadata> = HashMap::new();

        if self.table_map.contains_key(&table_id)
        {
            return Err(DbError::Other(format!("DbMetadata::add_new_table: map contains given table_id: '{}', Uuid::new gave the same id", table_id)));
        }

        create_dirs_if_not_exist(&create_dir_path(&table_id, table_name)).await?;

        for col in columns
        {
            let col_name = String::from(col.c_name());
            let file_path = create_file_path(&table_id, &table_name, &col_name, file_idx);

            create_file(&file_path).await?;

            col_map.insert(
                col_name.clone(), 
                ColMetadata { 
                    c_name: col_name.clone(), 
                    c_type: col.c_type(), 
                    c_files:  vec![
                            file_path   
                        ]
                }
            );
        }

        self.table_map.insert(
            table_id, 
            TableMetadata {
            table_name: String::from(table_name),
            columns: col_map
        });
        self.table_count += 1;

        Ok(table_id)
    }

    // ###################################################################### 
    // ############################ GETTERS #################################
    // ###################################################################### 

}


#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct TableMetadata
{
    table_name: TableName,
    columns: HashMap<ColumnName, ColMetadata>,
    // row_count: u32 // We should for given table remember how many rows it has
}

impl TableMetadata
{
    fn into_table_schema(&self) -> TableSchema
    {
        let mut t_schema = TableSchema::new(&self.table_name);

        for (col_name, col_meta) in &self.columns
        {
            t_schema.push_col(&Column::new(col_name, &col_meta.c_type));
        }

        t_schema
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ColMetadata
{
    // table_name: TableName, // ???
    c_name: ColumnName,
    c_type: LogicalColType,
    c_files: Vec<FilePath>
}

impl ColMetadata
{
    pub fn new(c_name: &str, c_type: LogicalColType) -> ColMetadata
    {
        ColMetadata { 
            c_name: String::from(c_name), 
            c_type, 
            c_files: Vec::new()
        }
    }

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
        if table_meta.columns.len() > MAX_COL_COUNT
        {
            return Err(DbError::SizeExceeded{
                msg: format!("DbMetadata::new: number of columns in table: '{}' is greater than MAX_COL_COUNT", &table_meta.table_name),
                max: MAX_COL_COUNT
            });
        }

        for (col_name, _) in &table_meta.columns
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

async fn create_dirs_if_not_exist(file_path: &str) -> std::io::Result<()>
{
    let path = Path::new(file_path);

    if let Some(parent_dir) = path.parent(){
        if let Some(grandparent_dir) = parent_dir.parent(){
            if !grandparent_dir.exists(){
                t_fs::create_dir(grandparent_dir).await?;
            }
        }

        if !parent_dir.exists(){
            t_fs::create_dir_all(parent_dir).await?;
        }
    }

    Ok(())
}

async fn create_file(file_path: &str) -> std::io::Result<()>
{
    let path = Path::new(file_path);

    if !path.exists(){
        let f = t_fs::File::create(path).await?;

        // We sync file to transfer it to disk
        f.sync_data().await;
    }

    // We need to sync directory to ensure write happended
    if let Some(parent_dir) = path.parent(){
        let dir = t_fs::File::open(parent_dir).await?;

        dir.sync_data().await;
    }

    Ok(())
}

fn create_filepaths_for_all_columns(
        table_map: &mut HashMap<TableId, TableMetadata>,
) -> Result<(), DbError>
{
    for (table_id, table_meta) in table_map
    {
        let table_name = &table_meta.table_name;

        for (col_name, col_meta) in &mut table_meta.columns
        {
            if *col_name != col_meta.c_name
            {
                return Err(DbError::Other(format!("DbMetadata::new:: name: '{}' of column in hashmap KEY, is not the same as name: '{}' of column in hashmap VALUE (ColMetadata) in table: '{}'", col_name, col_meta.c_name, table_name)));
            }

            // At first we have only one file for each column, when we read
            // enough data, we will create another one if the first is full
            let file_idx = 0;
            let file_path = create_file_path(&table_id, table_name, col_name, file_idx);

            col_meta.c_files.push(file_path);
        }
    }
    Ok(())
}

fn create_file_path(
    table_id: &Uuid, 
    table_name: &str, 
    col_name: &str, 
    idx: usize
) -> String
{
    format!("{DB_DATA_DIR}/{table_id}_{table_name}/{col_name}_{idx}")
}

fn create_dir_path(
    table_id: &Uuid, 
    table_name: &str, 
) -> String
{
    format!("{DB_DATA_DIR}/{table_id}_{table_name}")
}


