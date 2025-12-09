use regex::Regex;
use serde;
use serde_json;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::{vec};
use tokio::fs as t_fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;
use log::{info, warn, debug, error};

use crate::db::constants::{
    FILE_PATH_REGEX, LogicalColType, MAX_ALLOWED_METADATA_CHANGES, MAX_COL_COUNT, MAX_COL_NAME_LEN,
};
use crate::db::errors::DbError;
use crate::db::manager::messages::{
    CopyQData, QueryData, SelectQData,
};
use crate::db::storage::col_data::ColData;
use crate::db::storage::col_header::ColHeader;
use crate::schemas::column::{Column, DataColumn, Int64Column, VarcharColumn};
use crate::schemas::query::{
    AllowedQuery, Query, QueryResult, QueryStatus, QueryTableName,
};
use crate::schemas::table::{ShallowTable, TableSchema};

#[cfg(test)]
#[path = "../tests/storage/test_metadata.rs"]
mod test_metadata;

type TableName = String;
pub type ColumnName = String;
type FilePath = String;
pub type TableId = Uuid;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DbMetadata {
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

impl DbMetadata {
    fn new(
        table_count: u16,
        table_map: HashMap<TableId, TableMetadata>,
        metadata_file_path: &str,
        data_dir_path: &str,
    ) -> Result<DbMetadata, DbError> {
        // TODO: add better checking if path is correct
        is_metadata_ok(table_count, &table_map, metadata_file_path, data_dir_path)?;

        let tables_states = TableState::new_map(&table_map);
        let table_name_to_id_map = create_table_name_to_id_map(&table_map);
        let db_meta = DbMetadata {
            table_count: table_count,
            tables_metadata: table_map,
            db_data_dir_path: String::from(data_dir_path),
            metadata_file_path: String::from(metadata_file_path),
            tables_states: tables_states,
            table_name_to_id_map: table_name_to_id_map,
            nbr_of_metadata_changes: 0,
        };

        // For each column we create a filepath: DB_DIR/table_name/col_name_0
        // Not FILE, file will be created when adding new data
        // These paths will be created with first COPY query
        // db_meta.create_filepaths_for_all_columns()?;

        Ok(db_meta)
    }

    pub fn new_empty(metadata_file_path: &str, data_dir_path: &str) -> Result<DbMetadata, DbError> {
        let table_count = 0;
        let table_map = HashMap::new();

        DbMetadata::new(table_count, table_map, metadata_file_path, data_dir_path)
    }

    /// We do not encode metadata when saving to file.
    /// When saving we overwrite whole previous metadata file
    /// This method is run when server ends its execution.
    pub async fn save_to_file(&self) -> Result<(), DbError> {
        let mut f = t_fs::File::create(&self.metadata_file_path).await?;
        let buf =
            serde_json::to_vec(&self).or_else(|e| return Err(DbError::Other(e.to_string())))?;

        f.write_all(&buf[..]).await?;
        f.flush().await?;

        Ok(())
    }

    pub async fn read_from_file(metadata_path: &str) -> Result<DbMetadata, DbError> {

        let mut f = t_fs::File::open(metadata_path).await?;
        let mut buf = vec![];

        // We expect that metadata file can be read WHOLE to memory
        f.read_to_end(&mut buf).await?;

        let mut metadata: DbMetadata =
            serde_json::from_slice(&buf[..]).or_else(|e| return Err(DbError::IoError(e.into())))?;

        metadata.metadata_file_path = String::from(metadata_path);

        is_metadata_ok(
            metadata.table_count,
            &metadata.tables_metadata,
            &metadata.metadata_file_path,
            &metadata.db_data_dir_path,
        )?;

        let tables_state = TableState::new_map(&metadata.tables_metadata);

        metadata.tables_states = tables_state;
        metadata.table_name_to_id_map = create_table_name_to_id_map(&metadata.tables_metadata);
        metadata.nbr_of_metadata_changes = 0;

        Ok(metadata)
    }

    pub fn get_tables(&self) -> Vec<ShallowTable> {
        let mut tables: Vec<ShallowTable> = Vec::new();

        for (t_id, t_meta) in &self.tables_metadata {
            // If everything works correctly, we should have the same table_ids
            // in tables_states AND in tables_metadata
            let table_state = self.tables_states
                            .get(t_id)
                            .expect(&format!("DbMetadata::get_tables: Our DB somehow ended up in INVALID STATE, tables_states doesn't have id: '{}', while tables_metadata has such id", t_id));

            match table_state.delete_flag {
                DeleteFlag::NoDelete => tables.push(ShallowTable::new(t_id, &t_meta.table_name)),
                _ => (),
            }
        }

        tables
    }

    pub fn get_table_details(&self, table_id: &Uuid) -> Result<TableSchema, DbError> {
        if let Some(tab_meta) = self.tables_metadata.get(table_id) {
            let table_state = self.tables_states
                            .get(table_id)
                            .expect(&format!("DbMetadata::get_table_details: Our DB somehow ended up in INVALID STATE, tables_states doesn't have id: '{}', while tables_metadata has such id", table_id));
            match table_state.delete_flag {
                DeleteFlag::NoDelete => return Ok(tab_meta.into_table_schema()),
                DeleteFlag::DoDelete => {
                    return Err(DbError::NotFound(format!(
                        "Table with id: {}, not found in db.",
                        table_id
                    )));
                }
            }
        }
        Err(DbError::NotFound(format!(
            "Table with id: {}, not found in db.",
            table_id
        )))
    }

    pub fn mark_table_for_deletion(&mut self, table_id: &Uuid) -> Result<(), DbError> {
        if !self.tables_metadata.contains_key(table_id)
            || !self.tables_states.contains_key(table_id)
        {
            return Err(DbError::NotFound(format!(
                "Table with id: {} couldn't be deleted, since it's not in database",
                table_id
            )));
        }

        self.tables_states
            .get_mut(table_id)
            .unwrap() // will never panic because of previous checks
            .delete_flag = DeleteFlag::DoDelete;

        Ok(())
    }

    pub fn delete_table(&mut self, table_id: &Uuid) -> Result<TableMetadata, DbError> {
        let table_meta = self.tables_metadata.get(table_id);
        let table_state = self.tables_states.get(table_id);

        if let Some(_) = table_meta
            && let Some(t_state) = table_state
        {
            if t_state.n_queries_operating_on_table > 0 {
                return Err(DbError::Other(format!(
                    "Table with id: {} couldn't be deleted, since there are still: '{}' queries operating on it.",
                    table_id, t_state.n_queries_operating_on_table
                )));
            }
            if t_state.delete_flag != DeleteFlag::DoDelete {
                return Err(DbError::Other(format!(
                    "Table with id: {} couldn't be deleted, since it's not marked for deletion",
                    table_id
                )));
            }

            let t_meta = self.tables_metadata.remove(table_id).unwrap();
            self.tables_states.remove(table_id);
            self.table_name_to_id_map.remove(t_meta.table_name());
            self.table_count -= 1;

            return Ok(t_meta);
        }

        return Err(DbError::NotFound(format!(
            "Table with id: {} couldn't be deleted, since it's not in database",
            table_id
        )));
    }

    /// Function receives TableSchema and adds table with its columns to
    /// metadata structure, it **DOESN't create** dirs and files
    pub fn put_table(&mut self, table_schema: &TableSchema) -> Result<TableId, DbError> {
        let table_id = TableId::new_v4();

        // TODO: add hashmap that stores table_name: table_id so that we can
        // quickly check if table_name exists in db (since task description
        // requires tables to have unique names)
        if self.tables_metadata.contains_key(&table_id) {
            return Err(DbError::Other(format!(
                "DbMetadata::add_new_table: map contains given table_id: '{}', Uuid::new gave the same id, this shouldnt happen",
                table_id
            )));
        }

        let columns =
            table_schema_into_columns_vec(&table_schema, &table_id, &self.db_data_dir_path)?;

        // Only if we successfully create columns metadata we insert new table
        // to our metadata object
        self.tables_metadata.insert(
            table_id.clone(),
            TableMetadata::new(
                table_schema.name(),
                &table_id,
                columns,
                &create_dir_path(&self.db_data_dir_path, &table_id, table_schema.name()),
            ),
        );
        self.tables_states.insert(table_id, TableState::new());
        self.table_name_to_id_map
            .insert(String::from(table_schema.name()), table_id);
        self.table_count += 1;
        self.nbr_of_metadata_changes += 1;

        Ok(table_id)
    }

    pub fn plan_query_execution(&self, q: &mut Query) -> Result<QueryData, DbError> {
        self.authorize_query(q.query_def())?;
        q.update_status(QueryStatus::PLANNING);

        let q_id = q.id();
        let table_id = self.table_name_to_id_map.get(q.table_name()).unwrap();
        let table_meta = self.tables_metadata.get(table_id).unwrap().clone();

        match q.query_def() {
            AllowedQuery::SelectQ(_) => {
                return Ok(QueryData::SelectQ(SelectQData::new(*q_id, table_meta)));
            }
            AllowedQuery::CopyQ(c_q) => {
                return Ok(QueryData::CopyQ(CopyQData::new(
                    *q_id,
                    c_q.clone(),
                    table_meta,
                )));
            }
        }
    }

    /// Checks if there exists table that this query is for.
    /// If table exists, it checks if table is marked to be deleted
    /// Returns OK if table exists and is not marked to be deleted
    pub fn authorize_query(&self, query: &impl QueryTableName) -> Result<(), DbError> {
        let table_name = query.table_name();

        if let Some(table_id) = self.table_name_to_id_map.get(table_name) {
            // In put_table and delete_table we either insert or remove
            // given table from ALL maps, so this should always be ok
            if let Some(t_state) = self.tables_states.get(&table_id)
                && self.tables_metadata.contains_key(&table_id)
            {
                if t_state.delete_flag == DeleteFlag::NoDelete {
                    return Ok(());
                }
                return Err(DbError::NotFound(format!(
                    "SELECT query for table: '{}' ABORTED, table is already deleted",
                    table_name
                )));
            }
            return Err(DbError::InternalDbError(format!(
                "SELECT query for table: '{}' ABORTED, such table exists in table_name_to_id_map BUT NOT in tables_states, DB CORRUPTED",
                table_name
            )));
        }
        return Err(DbError::NotFound(format!(
            "SELECT query for table: '{}' ABORTED, such table does not exist in db",
            table_name
        )));
    }

    pub fn increase_nbr_of_queries_operating_on_table(
        &mut self,
        q: &impl QueryTableName,
    ) -> Result<(), DbError> {
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
        // TODO: add enum here
        table_id: Option<&Uuid>,
        table_name: Option<&str>,
    ) -> Result<(), DbError> {
        if let Some(table_id) = table_id {
            return self.do_queries_decrease(table_id);
        }

        if let Some(table_name) = table_name {
            let table_id = self.table_name_to_id_map.get(table_name);

            if let Some(table_id) = table_id {
                // Clone so that borrow checker doesnt complain about immutable
                // ref while having mutable ref
                return self.do_queries_decrease(&table_id.clone());
            } else {
                // Probably should be internal db erorr
                return Err(DbError::NotFound(format!(
                    "DbMetadata::decrease_nbr_of_queries_operating: provided table name: '{}' does not exist in database",
                    table_name
                )));
            }
        }
        // Probably should be internal db erorr
        return Err(DbError::Other(format!(
            "DbMetadata::decrease_nbr_of_queries_operating: both table_name and table_id are None"
        )));
    }

    pub fn lift_copy_lock_from_table(
        &mut self,
        table_id: &Uuid,
        is_copy: bool,
    ) -> Result<(), DbError> 
    {
        if is_copy {
            return DbMetadata::do_the_copy_lock_lifting(
                self.tables_states.get_mut(table_id)
            );
        }
        return Ok(());
    }

    pub fn acquire_copy_lock_for_table(
        &mut self, 
        table_id: &Uuid
    ) -> Result<bool, DbError>
    {
        match self.tables_states.get_mut(table_id)
        {
            Some(t_state) => {
                if t_state.copy_flag == CopyFlag::NoCopy
                {
                    t_state.copy_flag = CopyFlag::CopyInProgress;
                    return Ok(true);
                }
                return Ok(false);
            },
            None => {
                return Err(DbError::InternalDbError(
                    format!(
                        "DbMetadata::is_copy_lock_free - table_id: {}, is not present in table_states, db corrupted", table_id
                    )
                ));
            }
        }
    }

    fn do_the_copy_lock_lifting(
        table_state: Option<&mut TableState>
    ) -> Result<(), DbError> {
        // if we run this function we expect copy lock to be set
        // otherwise it means when we started execution of COPY QUERY we forgot
        // to set it thus this is corrupted DB state
        match table_state {
            Some(t_state) => match t_state.copy_flag {
                CopyFlag::NoCopy => {
                    return Err(DbError::InternalDbError(format!(
                        "DbMetadata::lift_copy_lock_from_table: COPY QUERY ended, however copy flag wasn't set to CopyInProgress, corrupted state of db"
                    )));
                }
                CopyFlag::CopyInProgress => {
                    t_state.copy_flag = CopyFlag::NoCopy;
                    return Ok(());
                }
            },
            None => {
                return Err(DbError::InternalDbError(format!(
                    "DbMetadata::lift_copy_lock_from_table: copy query, table_id not found, corrupted state of db"
                )));
            }
        }
    }

    pub fn append_newly_created_column_files(
        &mut self,
        table_id: &Uuid,
        latest_columns_file_ids: Vec<(String, u16)>,
    ) -> Result<(), DbError> {
        let table_meta = match self.tables_metadata.get_mut(table_id) {
            Some(val) => val,
            None => {
                return Err(DbError::InternalDbError(format!(
                    "DbMetadata::append_newly_created_column_files - givrn table id doesnt exist in tables metadata, db in corrupted state"
                )));
            }
        };

        let table_name = String::from(table_meta.table_name());

        if table_meta.columns.len() != latest_columns_file_ids.len() {
            return Err(DbError::InternalDbError(format!(
                "DbMetadata::append_newly_created_column_files - nbr of columns in table metadata ({}) is different than provided in latest_columns_file_ids ({})",
                table_meta.columns.len(),
                latest_columns_file_ids.len()
            )));
        }

        // Iterate over both vectors simultaneously and check if column names
        // match and if they do we can create and push new file paths
        for (col_meta, (col_name, last_id)) in table_meta
            .columns
            .iter_mut()
            .zip(latest_columns_file_ids.iter())
        {
            if col_meta.c_name != *col_name {
                return Err(DbError::InternalDbError(format!(
                    "DbMetadata::append_newly_created_column_files - column name mismatch: table has '{}' but provided '{}'",
                    col_meta.c_name, col_name
                )));
            }

            let last_id = *last_id as usize;
            let free_file_id = col_meta.get_first_free_col_file_id();

            for id in free_file_id..=last_id {
                col_meta.push_col_file(create_file_path(
                    &self.db_data_dir_path,
                    table_id,
                    &table_name,
                    &col_name,
                    id,
                ));
            }
        }

        return Ok(());
    }

    fn do_queries_decrease(&mut self, table_id: &Uuid) -> Result<(), DbError> {
        if let Some(t_state) = self.tables_states.get_mut(table_id) {
            if t_state.n_queries_operating_on_table > 0 {
                t_state.n_queries_operating_on_table -= 1;
                return Ok(());
            } else {
                return Err(DbError::Other(format!(
                    "DbMetadta::decrease_nbr_of_queries_operating_on_table - We wanted to decrease even though there are no queries operating on table"
                )));
            }
        } else {
            return Err(DbError::Other(format!(
                "DbMetadata::decrease_nbr_of_queries_operating_on_table - there is no table in db with id: {}",
                table_id
            )));
        }
    }

    pub fn is_enough_changes(&self) -> bool {
        self.nbr_of_metadata_changes >= MAX_ALLOWED_METADATA_CHANGES
    }

    pub fn reset_changes(&mut self) {
        self.nbr_of_metadata_changes = 0;
    }
    // ######################################################################
    // ############################ GETTERS #################################
    // ######################################################################
    pub fn get_table_id(&self, table_name: &str) -> Result<Uuid, DbError>
    {
        match self.table_name_to_id_map.get(table_name)
        {
            Some(id) => {
                return Ok(*id);
            },
            None => {
                return Err(DbError::InternalDbError(
                    format!("DbMetadata::get_table_id: no table with name: '{}' in db", table_name)
                ));
            }
        }
    }
    // ######################################################################
    // ##################### FILE HANDLING FUNCTIONS ########################
    // ######################################################################

    /// Function creates given directory, and its parents if they don't exist.
    /// <br> Function expects dir_path to be: **DB_DATA_DIR/table_name**
    /// <br> Otherwise it returns error.
    async fn create_dirs_if_not_exist(&self, dir_path: &str) -> std::io::Result<()> {
        // We should get path: DB_DATA_DIR/tableId_tableName
        // and we want to create dir: 'tableId_tableName'
        let correct_parent = Path::new(&self.db_data_dir_path);
        let path = Path::new(dir_path);

        if let Some(parent_dir) = path.parent() {
            if parent_dir != correct_parent {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "DbMetadata::create_dirs_if_not_exist - provided dir_path: {} doesnt have correct parent dir: {}",
                        dir_path, &self.db_data_dir_path
                    ),
                ));
            }

            // Even if parent dir does not exist yet, we create it here
            t_fs::create_dir_all(path).await?;
            return Ok(());
        }

        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "DbMetadata::create_dirs_if_not_exist - provided dir_path: {} doesnt have parent dir: {}",
                dir_path, &self.db_data_dir_path
            ),
        ));
    }

    async fn delete_dir_with_contents(&self, dir_path: &str) -> std::io::Result<()> {
        let dir_path = Path::new(dir_path);

        // Only folder with a parent can be deleted
        if let Some(parent_dir) = dir_path.parent() {
            // We want to ensure that we will delete only folders that have
            // their parent equal to DB_DATA_DIR
            if parent_dir == Path::new(&self.db_data_dir_path) {
                t_fs::remove_dir_all(dir_path).await?;
                return Ok(());
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "Provided dir path: '{:?}' does not have parent equal to: '{}'",
                dir_path, &self.db_data_dir_path
            ),
        ))
    }

    async fn create_file(&self, file_path: &str) -> std::io::Result<()> {
        // Path should be: DB_DATA_DIR/table_name/file
        let path = Path::new(file_path);

        if let Some(parent_dir) = path.parent() {
            if let Some(grandparent_dir) = parent_dir.parent() {
                if grandparent_dir != Path::new(&self.db_data_dir_path) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!(
                            "Provided file path: '{}' does not have grandparent equal to: '{}'",
                            file_path, &self.db_data_dir_path
                        ),
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
            format!(
                "Provided file path: '{}' does not have correct ancestors: '{}'",
                file_path, &self.db_data_dir_path
            ),
        ));
    }

    /// Should be run **always after is_metadata_ok function**, since it assumes
    /// that data it has is correct i.e. column names do not constain non-ASCII
    /// characters
    fn create_filepaths_for_all_columns(&mut self) -> Result<(), DbError> {
        let file_idx = 0;

        for (table_id, table_meta) in &mut self.tables_metadata {
            let table_name = &table_meta.table_name;

            for col_meta in &mut table_meta.columns {
                // At first we have only one FILE PATH (files will be created
                // later) for each column
                let file_path = create_file_path(
                    &self.db_data_dir_path,
                    &table_id,
                    table_name,
                    &col_meta.c_name,
                    file_idx,
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
enum DeleteFlag {
    DoDelete,
    NoDelete,
}

#[derive(Clone, PartialEq, Debug)]
enum CopyFlag {
    CopyInProgress,
    NoCopy,
}

#[derive(Clone)]
struct TableState {
    delete_flag: DeleteFlag,
    copy_flag: CopyFlag,
    n_queries_operating_on_table: u16,
}

impl TableState {
    fn new() -> TableState {
        TableState {
            delete_flag: DeleteFlag::NoDelete,
            copy_flag: CopyFlag::NoCopy,
            n_queries_operating_on_table: 0,
        }
    }

    fn new_map(tables_metadata: &HashMap<TableId, TableMetadata>) -> HashMap<TableId, TableState> {
        let mut tables_state: HashMap<TableId, TableState> = HashMap::new();
        for (table_id, _) in tables_metadata {
            tables_state.insert(
                *table_id,
                TableState {
                    delete_flag: DeleteFlag::NoDelete,
                    copy_flag: CopyFlag::NoCopy,
                    n_queries_operating_on_table: 0,
                },
            );
        }
        tables_state
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct TableMetadata {
    table_name: TableName,
    table_id: Uuid,
    columns: Vec<ColMetadata>,
    // row_count: u32,
    table_dir_path: String,
}

pub enum ColDataWrapper {
    IntColData(ColData<i64>),
    StrColData(ColData<String>),
}

impl TableMetadata {
    fn new(name: &str, id: &Uuid, columns: Vec<ColMetadata>, dir_path: &str) -> TableMetadata {
        TableMetadata {
            table_name: String::from(name),
            table_id: id.clone(),
            columns: columns,
            table_dir_path: String::from(dir_path),
        }
    }

    fn into_table_schema(&self) -> TableSchema {
        let mut t_schema = TableSchema::new(&self.table_name);

        for col_meta in &self.columns {
            t_schema.push_col(&Column::new(&col_meta.c_name, &col_meta.c_type));
        }

        t_schema
    }

    pub fn table_name(&self) -> &str {
        &self.table_name
    }

    pub fn all_column_files_are_empty(&self) -> Result<bool, DbError>
    {
        let mut presence_count: usize = 0;
        for col_meta in &self.columns
        {
            println!("col_name: {}", col_meta.c_name);
            if col_meta.column_files_present()
            {
                presence_count += 1;
            }
            println!("presence count: {}", presence_count);
        }

        println!("columns len: {}", self.columns.len());
        if presence_count != 0 && presence_count != self.columns.len()
        {
            return Err(DbError::InternalDbError(
                format!("TableMEtadata::are_all_column_files_empty - presence_count is not equal to columns len, this means that some columns have files and other don't, this cannot happen")
            ));
        }

        return Ok(presence_count == 0);
    }

    pub fn columns(&self) -> &Vec<ColMetadata> {
        &self.columns
    }

    pub fn table_id(&self) -> Uuid {
        self.table_id
    }

    pub fn get_column_name_to_index_map(&self) -> HashMap<String, usize> {
        self.columns
            .iter()
            .enumerate()
            .map(|(idx, col_meta)| (col_meta.c_name.clone(), idx))
            .collect()
    }

    // TODO: better name for this function
    pub fn create_col_data_vec(&self) -> Result<Vec<ColDataWrapper>, DbError> {
        let mut res_vec: Vec<ColDataWrapper> = vec![];

        for col_meta in &self.columns {
            // Column files ids start from 0, thus new column will have id equal
            // to the len of file vector
            let col_file_id = col_meta.c_files.len() as u16;
            let is_overflow = false;
            let initial_size_of_data = 0;
            let col_type = col_meta.c_type();

            let header = ColHeader::new(
                col_file_id,
                col_type,
                is_overflow,
                initial_size_of_data,
                String::from(col_meta.c_name()),
                &self.table_dir_path,
            )?;

            let col_data = match col_type {
                LogicalColType::INT64 => {
                    let x = ColData::<i64>::new(header)?;
                    ColDataWrapper::IntColData(x)
                }
                LogicalColType::VARCHAR => {
                    let x = ColData::<String>::new(header)?;
                    ColDataWrapper::StrColData(x)
                }
            };
            res_vec.push(col_data);
        }
        return Ok(res_vec);
    }

    pub async fn read_table(&self) -> Result<(QueryResult, i32), DbError> {
        let mut row_count: Option<i32> = None;
        let mut q_res = QueryResult::new(0, vec![]);

        for col_meta in &self.columns {
            let mut file_queue: VecDeque<&str> = VecDeque::new();

            for path in &col_meta.c_files {
                file_queue.push_back(path);
            }

            match col_meta.c_type {
                LogicalColType::INT64 => {
                    let col_data =
                        ColData::<i64>::read_from_file(file_queue, &self.table_dir_path).await?;

                    TableMetadata::check_row_count(
                        &mut row_count,
                        col_data.n_rows(),
                        &self.table_name,
                    )?;

                    let col_data = DataColumn::Int64(Int64Column::new(col_data.data()));

                    q_res.push_col_data(col_data);
                }
                LogicalColType::VARCHAR => {
                    let col_data =
                        ColData::<String>::read_from_file(file_queue, &self.table_dir_path).await?;

                    TableMetadata::check_row_count(
                        &mut row_count,
                        col_data.n_rows(),
                        &self.table_name,
                    )?;

                    let col_data = DataColumn::Varchar(VarcharColumn::new(col_data.data()));
                    q_res.push_col_data(col_data);
                }
            }
        }

        match row_count {
            None => {
                // Something is corrupted in our db, since in put_table endpoint
                // we do not allow table schemas without columns
                return Err(DbError::InternalDbError(format!(
                    "TableMetadata::read_table - final row_count is NONE, this means that table has zero columns, we do not allow that in our db"
                )));
            }
            Some(row_count) => {
                return Ok((q_res, row_count));
            }
        }
    }

    fn check_row_count(
        row_count: &mut Option<i32>,
        curr_col_rows: usize,
        table_name: &str,
    ) -> Result<(), DbError> {
        match row_count {
            None => {
                if curr_col_rows > i32::MAX as usize {
                    // This shouldnt happen if all our db files are not corrupted
                    return Err(DbError::InternalDbError(format!(
                        "TableMEtadata:: nbr of rows in table greater than i32"
                    )));
                }
                *row_count = Some(curr_col_rows as i32);
            }
            Some(val) => {
                if *val as usize != curr_col_rows {
                    // This shouldnt happen if all our db files are not corrupted
                    return Err(DbError::InternalDbError(format!(
                        "Table: '{}' has columns with different number of rows: {} != {}",
                        table_name, *val, curr_col_rows
                    )));
                }
            }
        }

        return Ok(());
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ColMetadata {
    c_name: ColumnName,
    c_type: LogicalColType,
    c_files: Vec<FilePath>,
}

impl ColMetadata {
    pub fn new(c_name: &str, c_type: LogicalColType) -> ColMetadata {
        ColMetadata {
            c_name: String::from(c_name),
            c_type,
            c_files: Vec::new(),
        }
    }

    pub fn column_files_present(&self) -> bool
    {
        !self.c_files.is_empty()
    }

    pub fn c_name(&self) -> &str {
        &self.c_name
    }

    pub fn c_type(&self) -> LogicalColType {
        self.c_type
    }

    pub fn get_first_free_col_file_id(&self) -> usize {
        self.c_files.len()
    }

    pub fn push_col_file(&mut self, file_path: String) {
        self.c_files.push(file_path);
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
) -> Result<(), DbError> {
    let re = Regex::new(FILE_PATH_REGEX).unwrap();
    if !re.is_match(&metadata_file_path) {
        return Err(DbError::Other(format!(
            "DbMetadata::new: file_path: '{}', does not satisfy regex: '{}'",
            metadata_file_path, FILE_PATH_REGEX
        )));
    }

    if !re.is_match(&data_dir_path) {
        return Err(DbError::Other(format!(
            "DbMetadata::new: data_dir_path: '{}', does not satisfy regex: '{}'",
            data_dir_path, FILE_PATH_REGEX
        )));
    }

    if table_count as usize != table_map.len() {
        return Err(DbError::SizeMismatch {
            msg: format!("DbMetadata::new: table_count has diff len than table_cols map"),
            size_1: table_count as usize,
            size_2: table_map.len(),
        });
    }

    for (_, table_meta) in table_map {
        if table_meta.columns.len() > MAX_COL_COUNT {
            return Err(DbError::SizeExceeded {
                msg: format!(
                    "DbMetadata::new: number of columns in table: '{}' is greater than MAX_COL_COUNT",
                    &table_meta.table_name
                ),
                max: MAX_COL_COUNT,
            });
        }

        are_columns_ok(&table_meta.columns, &table_meta.table_name)?;
    }
    Ok(())
}

fn are_columns_ok(columns: &Vec<ColMetadata>, table_name: &str) -> Result<(), DbError> {
    for col_meta in columns {
        let col_name = &col_meta.c_name;
        if col_name.len() > MAX_COL_NAME_LEN {
            return Err(DbError::SizeExceeded {
                msg: format!(
                    "DbMetadata::new: column: '{}' length exceeds MAX_COL_NAME_LEN ",
                    col_name
                ),
                max: MAX_COL_NAME_LEN,
            });
        }

        if !col_name.is_ascii() {
            return Err(DbError::InvalidColumnName {
                msg: format!(
                    "In table: '{}', column name has non-ASCII characters",
                    table_name
                ),
                name: String::from(col_name),
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
    idx: usize,
) -> String {
    let dir_path = create_dir_path(db_data_dir_path, table_id, table_name);
    format!("{dir_path}/{col_name}_{idx}")
}

/// Creates dir path: {db_data_dir}/{table_id}\_{table_name}
fn create_dir_path(db_data_dir_path: &str, table_id: &Uuid, table_name: &str) -> String {
    format!("{db_data_dir_path}/{table_id}_{table_name}")
}

// TODO: probably we should make this not a function, but TableSchema method !!!
fn table_schema_into_columns_vec(
    schema: &TableSchema,
    table_id: &Uuid,
    db_data_dir_path: &str,
) -> Result<Vec<ColMetadata>, DbError> {
    let file_idx = 0;
    let mut columns: Vec<ColMetadata> = Vec::new();

    // If we have duplicate col_names we will just overwrite previously added column
    for col in schema.columns() {
        let col_name = String::from(col.c_name());

        if !col_name.is_ascii() {
            let msg = format!(
                "DbManager::put_table::table_schema_into_col_metadata - provided table has column: '{}', that has non ASCII characters, creating table ABORTED",
                col_name
            );

            return Err(DbError::InvalidColumnName {
                msg: msg,
                name: col_name,
            });
        }
        if columns.iter().any(|c| c.c_name == col_name) {
            return Err(DbError::InvalidColumnName {
                msg: format!(
                    "Provided table: '{}' doesn't have UNIQUE column names, creating table ABORTED",
                    schema.name()
                ),
                name: col_name,
            });
        }

        // File paths will be created when first copy query comes
        // let file_path = create_file_path(
        //     db_data_dir_path,
        //     table_id,
        //     schema.name(),
        //     &col_name,
        //     file_idx,
        // );

        columns.push(ColMetadata {
            c_name: col_name.clone(),
            c_type: col.c_type(),
            c_files: vec![],
        });
    }

    if columns.is_empty() {
        return Err(DbError::WrongSize(format!(
            "Table: {} has 0 columns, we do not allow that in our DB",
            schema.name()
        )));
    }

    Ok(columns)
}

fn create_table_name_to_id_map(
    table_map: &HashMap<TableId, TableMetadata>,
) -> HashMap<TableName, TableId> {
    let mut name_to_id_map = HashMap::new();
    for (tab_id, tab_meta) in table_map {
        name_to_id_map.insert(tab_meta.table_name.clone(), tab_id.clone());
    }

    name_to_id_map
}
