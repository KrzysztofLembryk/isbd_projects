use std::collections::HashMap;
use std::path::Path;
use std::{result, vec};
use regex::Regex;
use serde;
use serde_json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;
use tokio::fs as t_fs;

use crate::db::errors::{DbError};
use crate::db::constants::{MAX_COL_NAME_LEN, MAX_COL_COUNT, LogicalColType, FILE_PATH_REGEX, MAX_ALLOWED_METADATA_CHANGES};
use crate::db::manager::messages::{CopyQData, QueryData, SelectQData};
use crate::schemas::table::{TableSchema, ShallowTable};
use crate::schemas::query::{AllowedQuery, Query, QueryResult, QueryStatus, QueryTableName};
use crate::schemas::column::{Column};

#[cfg(test)]
#[path = "../tests/storage/test_metadata.rs"]
mod test_metadata;

type TableName = String;
pub type ColumnName = String;
type FilePath = String;
pub type TableId = Uuid;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DbMetadata
{
    /// In DbMetadata struct we will store all file names, dirs, etc 
    /// Each table has its own directory: 'table_name/'
    /// Thus we cannot have two tables with the same name.
    /// -- table_count: variable that remembers how many tables in db we have
    /// -- tables_metadata: hash map in which for given table_id we store all 
    ///                     its columns metadata
    /// -- db_data_dir_path: path to dir in which we store all database folders
    table_count: u16,
    tables_metadata: HashMap<TableId, TableMetadata>,
    db_data_dir_path: String, 
    #[serde(skip)]
    metadata_file_path: String,
    #[serde(skip)]
    tables_states: HashMap<TableId, TableState>,
    #[serde(skip)]
    table_name_to_id_map: HashMap<TableName, TableId>,
    #[serde(skip)]
    nbr_of_metadata_changes: u16,
}

impl DbMetadata  
{
    fn new(
        table_count: u16,
        table_map: HashMap<TableId, TableMetadata>,
        metadata_file_path: &str,
        data_dir_path: &str,
    ) -> Result<DbMetadata, DbError>
    {
        // TODO: add better checking if path is correct
        is_metadata_ok(
            table_count, 
            &table_map, 
            metadata_file_path,
            data_dir_path
        )?;

        let tables_states = TableState::new_map(&table_map);
        let table_name_to_id_map = create_table_name_to_id_map(&table_map);
        let mut db_meta = DbMetadata {
            table_count: table_count,
            tables_metadata: table_map,
            db_data_dir_path: String::from(data_dir_path),
            metadata_file_path: String::from(metadata_file_path),
            tables_states: tables_states,
            table_name_to_id_map: table_name_to_id_map,
            nbr_of_metadata_changes: 0
        };

        // For each column we create a filepath: DB_DIR/table_name/col_name_0
        // Not FILE, file will be created when adding new data
        db_meta.create_filepaths_for_all_columns()?;

        Ok(db_meta)
    }

    pub fn new_empty(
        metadata_file_path: &str, 
        data_dir_path: &str
    ) -> Result<DbMetadata, DbError>
    {
        let table_count = 0;
        let table_map = HashMap::new();

        DbMetadata::new(
            table_count, 
            table_map, 
            metadata_file_path, 
            data_dir_path
        )
    }

    /// We do not encode metadata when saving to file.
    /// When saving we overwrite whole previous metadata file
    /// This method is run when server ends its execution.
    pub async fn save_to_file(&self) -> Result<(), DbError>
    {
        let mut f = t_fs::File::create(&self.metadata_file_path).await?;
        let buf = serde_json::to_vec(&self)
                            .or_else(|e| 
                                return Err(DbError::Other(e.to_string()))
                            )?;

        f.write_all(&buf[..]).await?;
        f.flush().await?;

        Ok(())
    }
    
    pub async fn read_from_file(
        metadata_path: &str, 
    ) -> Result<DbMetadata, DbError>
    {
        let mut f = t_fs::File::open(metadata_path).await?;
        let mut buf = vec![]; 

        // We expect that metadata file can be read WHOLE to memory
        f.read_to_end(&mut buf).await?;

        let mut metadata: DbMetadata = serde_json::from_slice(&buf[..])
                                .or_else(|e| 
                                    return Err(DbError::IoError(e.into()))
                                )?;

        metadata.metadata_file_path = String::from(metadata_path);

        is_metadata_ok(
            metadata.table_count, 
            &metadata.tables_metadata, 
            &metadata.metadata_file_path,
            &metadata.db_data_dir_path
        )?;

        let tables_state = TableState::new_map(&metadata.tables_metadata);

        metadata.tables_states = tables_state;
        metadata.table_name_to_id_map = create_table_name_to_id_map(&metadata.tables_metadata);
        metadata.nbr_of_metadata_changes = 0;

        Ok(metadata)
    }

    pub fn get_tables(&self) -> Vec<ShallowTable>
    {
        let mut tables: Vec<ShallowTable> = Vec::new();

        for (t_id, t_meta) in &self.tables_metadata
        {
            // If everything works correctly, we should have the same table_ids
            // in tables_states AND in tables_metadata
            let table_state = self.tables_states
                            .get(t_id)
                            .expect(&format!("DbMetadata::get_tables: Our DB somehow ended up in INVALID STATE, tables_states doesn't have id: '{}', while tables_metadata has such id", t_id));
            
            match table_state.delete_flag
            {
                DeleteFlag::NoDelete => tables.push(
                                    ShallowTable::new(t_id, &t_meta.table_name)
                                ),
                _ => ()
            }
        }

        tables
    }

    pub fn get_table_details(
        &self, 
        table_id: &Uuid
    ) -> Result<TableSchema, DbError>
    {
        if let Some(tab_meta) = self.tables_metadata.get(table_id)
        {
            let table_state = self.tables_states
                            .get(table_id)
                            .expect(&format!("DbMetadata::get_table_details: Our DB somehow ended up in INVALID STATE, tables_states doesn't have id: '{}', while tables_metadata has such id", table_id));
            match table_state.delete_flag
            {
                DeleteFlag::NoDelete => return Ok(tab_meta.into_table_schema()),
                DeleteFlag::DoDelete => return Err(DbError::NotFound(format!("Table with id: {}, not found in db.", table_id)))
            }
        }
        Err(DbError::NotFound(format!("Table with id: {}, not found in db.", table_id)))
    }

    pub fn mark_table_for_deletion(
        &mut self, 
        table_id: &Uuid
    ) -> Result<(), DbError>
    {
        if !self.tables_metadata.contains_key(table_id)
            || !self.tables_states.contains_key(table_id)
        {
            return Err(DbError::NotFound(format!("Table with id: {} couldn't be deleted, since it's not in database", table_id)));
        }

        self.tables_states
            .get_mut(table_id)
            .unwrap() // will never panic because of previous checks
            .delete_flag = DeleteFlag::DoDelete;

        Ok(())
    }

    pub fn delete_table(
        &mut self, 
        table_id: &Uuid
    ) -> Result<TableMetadata, DbError>
    {
        let table_meta = self.tables_metadata.get(table_id);
        let table_state = self.tables_states.get(table_id);

        if let Some(_) = table_meta 
        && let Some(t_state) = table_state
        {
            if t_state.n_queries_operating_on_table > 0
            {
                return Err(DbError::Other(format!("Table with id: {} couldn't be deleted, since there are still: '{}' queries operating on it.", table_id, t_state.n_queries_operating_on_table)));
            }
            if t_state.delete_flag != DeleteFlag::DoDelete
            {
                return Err(DbError::Other(format!("Table with id: {} couldn't be deleted, since it's not marked for deletion", table_id)));
            }

            let t_meta = self.tables_metadata.remove(table_id).unwrap();
            self.tables_states.remove(table_id);
            self.table_name_to_id_map.remove(t_meta.table_name());

            return Ok(t_meta);
        }

        return Err(DbError::NotFound(format!("Table with id: {} couldn't be deleted, since it's not in database", table_id)));
    }

    /// Function receives TableSchema and adds table with its columns to 
    /// metadata structure, it **DOESN't create** dirs and files
    pub fn put_table(
        &mut self, 
        table_schema: &TableSchema
    ) -> Result<TableId, DbError>
    {
        let table_id = TableId::new_v4();

        // TODO: add hashmap that stores table_name: table_id so that we can
        // quickly check if table_name exists in db (since task description 
        // requires tables to have unique names)
        if self.tables_metadata.contains_key(&table_id)
        {
            return Err(DbError::Other(format!("DbMetadata::add_new_table: map contains given table_id: '{}', Uuid::new gave the same id, this shouldnt happen", table_id)));
        }

        let columns = table_schema_into_columns_vec(
            &table_schema, 
            &table_id, 
            &self.db_data_dir_path,
        )?;

        // Only if we successfully create columns metadata we insert new table
        // to our metadata object
        self.tables_metadata.insert(
            table_id.clone(), 
            TableMetadata::new(table_schema.name(), &table_id, columns) 
        );
        self.tables_states.insert(table_id, TableState::new());
        self.table_name_to_id_map.insert(
            String::from(table_schema.name()), 
            table_id
        );
        self.table_count += 1;
        self.nbr_of_metadata_changes += 1;

        Ok(table_id)
    }

    pub fn plan_query_execution(
        &self, 
        q: &mut Query,
    ) -> Result<QueryData, DbError>
    {
        self.authorize_query(q.query_def())?;
        q.update_status(QueryStatus::PLANNING);

        let q_id = q.id();
        let table_id = self.table_name_to_id_map.get(q.table_name()).unwrap();
        let table_meta = self.tables_metadata.get(table_id).unwrap().clone();

        match q.query_def()
        {
            AllowedQuery::SelectQ(_) => {
                return Ok(QueryData::SelectQ(
                    SelectQData::new(*q_id, table_meta)
                ));
            },
            AllowedQuery::CopyQ(c_q) => {
                return Ok(QueryData::CopyQ(
                    CopyQData::new(*q_id, c_q.clone(), table_meta)
                ));
            }
        }
    }

    /// Checks if there exists table that this query is for.
    /// If table exists, it checks if table is marked to be deleted 
    /// Returns OK if table exists and is not marked to be deleted
    pub fn authorize_query(
        &self, 
        query: &impl QueryTableName
    ) -> Result<(), DbError>
    {
        let table_name = query.table_name();

        if let Some(table_id) = self.table_name_to_id_map
                                    .get(table_name)
        {
            // In put_table and delete_table we either insert or remove
            // given table from ALL maps, so this should always be ok
            if let Some(t_state) = self.tables_states.get(&table_id) 
                && self.tables_metadata.contains_key(&table_id)
            {
                if t_state.delete_flag == DeleteFlag::NoDelete
                {
                    return Ok(());
                }
                return Err(DbError::NotFound(format!("SELECT query for table: '{}' ABORTED, table is already deleted", table_name)));
            }
            return Err(DbError::InternalDbError(format!("SELECT query for table: '{}' ABORTED, such table exists in table_name_to_id_map BUT NOT in tables_states, DB CORRUPTED", table_name)));
        }
        return Err(DbError::NotFound(format!("SELECT query for table: '{}' ABORTED, such table does not exist in db", table_name)));
    }

    pub fn increase_nbr_of_queries_operating_on_table(
        &mut self, 
        q: &impl QueryTableName
    ) -> Result<(), DbError>
    {
        self.authorize_query(q)?;

        let table_name = q.table_name();

        // We do unwrap here since authorize query checked that these vals exist
        let table_id = self.table_name_to_id_map.get(table_name).unwrap();
        let t_state = self.tables_states.get_mut(table_id).unwrap();

        t_state.n_queries_operating_on_table += 1;

        Ok(())
    }

    pub fn decrease_nbr_of_queries_operating_on_table(
        &mut self, 
        table_id: &Uuid
    ) -> Result<(), DbError>
    {
        // TODO: add proper error handling
        if let Some(t_state) = self.tables_states.get_mut(table_id)
        {
            if t_state.n_queries_operating_on_table > 0
            {
                t_state.n_queries_operating_on_table -= 1;
                return Ok(());
            }
            else 
            {
                return Err(DbError::Other(format!("DbMetadta::decrease_nbr_of_queries_operating_on_table - We wanted to decrease even though there are no queries operating on table")));
            }
        }
        else 
        {
            return Err(DbError::Other(format!("DbMetadata::decrease_nbr_of_queries_operating_on_table - there is no table in db with id: {}", table_id)));
        }
    }

    pub fn is_enough_changes(&self) -> bool
    {
        self.nbr_of_metadata_changes >= MAX_ALLOWED_METADATA_CHANGES
    }

    pub fn reset_changes(&mut self)
    {
        self.nbr_of_metadata_changes = 0;
    }
    // ###################################################################### 
    // ############################ GETTERS #################################
    // ###################################################################### 

    // ###################################################################### 
    // ##################### FILE HANDLING FUNCTIONS ########################
    // ###################################################################### 

    /// Function creates given directory, and its parents if they don't exist.
    /// <br> Function expects dir_path to be: **DB_DATA_DIR/table_name**
    /// <br> Otherwise it returns error.
    async fn create_dirs_if_not_exist(
        &self, 
        dir_path: &str
    ) -> std::io::Result<()>
    {
        // We should get path: DB_DATA_DIR/tableId_tableName
        // and we want to create dir: 'tableId_tableName'
        let correct_parent = Path::new(&self.db_data_dir_path);
        let path = Path::new(dir_path);

        if let Some(parent_dir) = path.parent(){

            if parent_dir != correct_parent
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("DbMetadata::create_dirs_if_not_exist - provided dir_path: {} doesnt have correct parent dir: {}", dir_path, &self.db_data_dir_path)
                ));
            }

            // Even if parent dir does not exist yet, we create it here 
            t_fs::create_dir_all(path).await?;
            return Ok(())
        }

        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("DbMetadata::create_dirs_if_not_exist - provided dir_path: {} doesnt have parent dir: {}", dir_path, &self.db_data_dir_path)
        ));
    }

    async fn delete_dir_with_contents(
        &self, 
        dir_path: &str
    ) -> std::io::Result<()> 
    {
        let dir_path = Path::new(dir_path);

        // Only folder with a parent can be deleted
        if let Some(parent_dir) = dir_path.parent() 
        {
            // We want to ensure that we will delete only folders that have 
            // their parent equal to DB_DATA_DIR
            if parent_dir == Path::new(&self.db_data_dir_path)
            {
                t_fs::remove_dir_all(dir_path).await?;
                return Ok(());
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput, 
            format!("Provided dir path: '{:?}' does not have parent equal to: '{}'", dir_path, &self.db_data_dir_path)
        ))
    }

    async fn create_file(&self, file_path: &str) -> std::io::Result<()>
    {
        // Path should be: DB_DATA_DIR/table_name/file
        let path = Path::new(file_path);

        if let Some(parent_dir) = path.parent()
        {
            if let Some(grandparent_dir) = parent_dir.parent()
            {
                if grandparent_dir != Path::new(&self.db_data_dir_path)
                {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput, 
                        format!("Provided file path: '{}' does not have grandparent equal to: '{}'", file_path, &self.db_data_dir_path)
                    ));
                }

                let f = t_fs::File::create(path).await?;

                // We sync file to transfer it to disk
                f.sync_data().await?;

                // We need to sync directory to ensure write happended
                let f = t_fs::File::open(parent_dir).await?;

                f.sync_data().await?;

                return Ok(());
            }
        }

        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput, 
            format!("Provided file path: '{}' does not have correct ancestors: '{}'", file_path, &self.db_data_dir_path)
        ));
    }

    /// Should be run **always after is_metadata_ok function**, since it assumes
    /// that data it has is correct i.e. column names do not constain non-ASCII
    /// characters
    fn create_filepaths_for_all_columns(
        &mut self,
    ) -> Result<(), DbError>
    {
        let file_idx = 0;

        for (table_id, table_meta) in &mut self.tables_metadata
        {
            let table_name = &table_meta.table_name;

            for col_meta in &mut table_meta.columns
            {
                // At first we have only one FILE PATH (files will be created 
                // later) for each column 
                let file_path = create_file_path(
                    &self.db_data_dir_path,
                    &table_id, 
                    table_name, 
                    &col_meta.c_name, 
                    file_idx
                );

                col_meta.c_files.push(file_path);
            }
        }
        Ok(())
    }

}

// ############################################################################
// ############################ HELPER STRUCTS ################################
// ############################################################################

#[derive(Clone, PartialEq, Debug)]
enum DeleteFlag
{
    DoDelete,
    NoDelete
}

#[derive(Clone)]
struct TableState
{
    delete_flag: DeleteFlag,
    n_queries_operating_on_table: u16
}

impl TableState
{
    fn new() -> TableState
    {
        TableState { 
            delete_flag: DeleteFlag::NoDelete, 
            n_queries_operating_on_table: 0
        }
    }

    fn new_map(
        tables_metadata: &HashMap<TableId, TableMetadata>
    ) -> HashMap<TableId, TableState>
    {
        let mut tables_state: HashMap<TableId, TableState> = HashMap::new();
        for (table_id, _) in tables_metadata
        {
            tables_state.insert(*table_id, TableState { 
                delete_flag: DeleteFlag::NoDelete, n_queries_operating_on_table: 0
            });
        }
        tables_state
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct TableMetadata
{
    table_name: TableName,
    table_id: Uuid,
    columns: Vec<ColMetadata>,
    // row_count: u32 // TODO: Table should remember how many rows it has
}

impl TableMetadata
{
    fn new(
        name: &str, 
        id: &Uuid, 
        columns: Vec<ColMetadata>
    ) -> TableMetadata
    {
        TableMetadata {
        table_name: String::from(name),
        table_id: id.clone(),
        columns: columns
        }
    }

    fn into_table_schema(&self) -> TableSchema
    {
        let mut t_schema = TableSchema::new(&self.table_name);

        for col_meta in &self.columns
        {
            t_schema.push_col(&Column::new(&col_meta.c_name, &col_meta.c_type));
        }

        t_schema
    }

    pub fn table_name(&self) -> &str
    {
        &self.table_name
    }

    pub fn table_id(&self) -> &Uuid
    {
        &self.table_id
    }

    pub fn read_table(&self) -> QueryResult
    {
        let mut row_count: i32 = 0;
        let mut q_res = QueryResult::new(row_count, vec![]);

        for col_meta in &self.columns
        {

        }

        todo!("implement read_table")
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct ColMetadata
{
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

fn is_metadata_ok(
        table_count: u16,
        table_map: &HashMap<TableId, TableMetadata>,
        metadata_file_path: &str,
        data_dir_path: &str,
) -> Result<(), DbError>
{
    let re = Regex::new(FILE_PATH_REGEX).unwrap();
    if !re.is_match(&metadata_file_path) 
    {
        return Err(DbError::Other(format!("DbMetadata::new: file_path: '{}', does not satisfy regex: '{}'", metadata_file_path, FILE_PATH_REGEX)));
    }

    if !re.is_match(&data_dir_path) 
    {
        return Err(DbError::Other(format!("DbMetadata::new: data_dir_path: '{}', does not satisfy regex: '{}'", data_dir_path, FILE_PATH_REGEX)));
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

        are_columns_ok(&table_meta.columns, &table_meta.table_name)?;
    }
    Ok(())
}

fn are_columns_ok(
    columns: &Vec<ColMetadata>,
    table_name: &str
) -> Result<(), DbError>
{
    for col_meta in columns
    {
        let col_name = &col_meta.c_name;
        if col_name.len() > MAX_COL_NAME_LEN
        {
            return Err(DbError::SizeExceeded{
                msg: format!("DbMetadata::new: column: '{}' length exceeds MAX_COL_NAME_LEN ", col_name),
                max: MAX_COL_NAME_LEN
            });
        }

        if !col_name.is_ascii()
        {
            return Err(DbError::InvalidColumnName { 
                msg: format!("In table: '{}', column name has non-ASCII characters", table_name), 
                name: String::from(col_name)
            });
        }
    }
    Ok(())
}

/// Creates file path: {db_data_dir}/{table_id}\_{table_name}/{col_name}\_{idx}
fn create_file_path(
    db_data_dir_path: &str,
    table_id: &Uuid, 
    table_name: &str, 
    col_name: &str, 
    idx: usize
) -> String
{
    format!("{db_data_dir_path}/{table_id}_{table_name}/{col_name}_{idx}")
}

/// Creates dir path: {db_data_dir}/{table_id}\_{table_name}
fn create_dir_path(
    db_data_dir_path: &str,
    table_id: &Uuid, 
    table_name: &str, 
) -> String
{
    format!("{db_data_dir_path}/{table_id}_{table_name}")
}

// TODO: probably we should make this not a function, but TableSchema method !!!
fn table_schema_into_columns_vec(
    schema: &TableSchema, 
    table_id: &Uuid,
    db_data_dir_path: &str,
) -> Result<Vec<ColMetadata>, DbError>
{
    let file_idx = 0;
    let mut columns: Vec<ColMetadata> = Vec::new();

    // If we have duplicate col_names we will just overwrite previously added column
    for col in schema.columns()
    {
        let col_name = String::from(col.c_name());

        if !col_name.is_ascii()
        {
            let msg = format!("DbManager::put_table::table_schema_into_col_metadata - provided table has column: '{}', that has non ASCII characters, creating table ABORTED", col_name);

            return Err(DbError::InvalidColumnName{
                    msg: msg, 
                    name: col_name
                    });
        }
        if columns.iter().any(|c| c.c_name == col_name)
        {
            return Err(DbError::InvalidColumnName { msg: format!("Provided table: '{}' doesn't have UNIQUE column names, creating table ABORTED", schema.name()), name: col_name });
        }

        let file_path = create_file_path(
            db_data_dir_path, 
            table_id, 
            schema.name(),
            &col_name, 
            file_idx
        );

        columns.push(
            ColMetadata { 
                c_name: col_name.clone(), 
                c_type: col.c_type(), 
                c_files:  vec![
                        file_path   
                    ]
            }
        );
    }
    Ok(columns)
}

fn create_table_name_to_id_map(
    table_map: &HashMap<TableId, TableMetadata>
) -> HashMap<TableName, TableId>
{
    let mut name_to_id_map = HashMap::new();
    for (tab_id, tab_meta) in table_map
    {
        name_to_id_map.insert(tab_meta.table_name.clone(), tab_id.clone());
    }

    name_to_id_map
}
