use std::collections::{HashMap, HashSet};
use std::usize;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::db::constants::{BATCH_SIZE, CSV_DELIM, COPY_QUERY_NAME, 
FOR_TESTS_DO_LONG_QUERY_EXECUTION, FOR_TESTS_QUERY_EXECUTION_TIME};
use crate::db::errors::DbError;
use crate::db::manager::messages::{BaseQueryDataInfo, CopyQData, DbCmd, DbWorkerMsg, QueryCompletionMsg, QueryData, QueryFailureMsg, SelectQData, WorkerMsgRes}; 
use crate::db::storage::metadata::{ColDataWrapper, TableMetadata};
use crate::schemas::column::{DataColumn, Int64Column, VarcharColumn};
use crate::schemas::query::{CopyQuery, QueryResult, QueryTableName};
use crate::schemas::error::{MultipleProblemsError};
use uuid::Uuid;

use log::{warn, debug};

pub struct QueryWorker
{
    id: usize,
    tx_to_db: UnboundedSender<DbCmd>,
    rx: UnboundedReceiver<DbWorkerMsg>
}

impl QueryWorker
{
    pub fn new(
        id: usize, 
        tx: UnboundedSender<DbCmd>, 
        rx: UnboundedReceiver<DbWorkerMsg>
    ) -> QueryWorker
    {
        QueryWorker { id, tx_to_db: tx, rx }
    }

    pub async fn run(&mut self)
    {
        while let Some(msg) = self.rx.recv().await
        {
            match msg
            {
                DbWorkerMsg::ExecQuery(worker_id, q_data) => {
                    if worker_id == self.id
                    {
                        // simulating long query execution
                        if FOR_TESTS_DO_LONG_QUERY_EXECUTION
                        {
                            warn!("TEST MODE ENABLED: QueryWorker sleeps for {}s before executing query", FOR_TESTS_QUERY_EXECUTION_TIME);
                            tokio::time::sleep(tokio::time::Duration::from_secs(FOR_TESTS_QUERY_EXECUTION_TIME)).await;
                        }
                        self.execute_query(q_data).await;
                    }
                    else // worker_id != self.id
                    {
                        self.send_msg_to_db(
                            DbWorkerMsg::InternalError(
                                worker_id, 
                                QueryFailureMsg::new(
                                    q_data.query_id(), 
                                    q_data.table_id(), 
                                    MultipleProblemsError::new_with_one_problem(
                                        &format!("QueryWorker::run: Worker: '{}' got message with wrong worker id: '{}'", self.id, worker_id)
                                        , 
                                        "In proper functioning db this should never happen, there is probably a bug somewhere"
                                    )
                        )));
                    }
                },
                DbWorkerMsg::Shutdown => {
                    debug!("Worker '{}' is shutting down", self.id);
                    break;
                },
                _ => {
                    warn!("Worker '{}' got unsupported msg, Doing nothig", self.id);
                }
            }
        }
    }

    async fn execute_query(&self, q_data: QueryData)
    {
        let worker_id = self.id;
        match q_data
        {
            QueryData::SelectQ(s_q) => {
                debug!("Worker '{}' got SELECT QUERY: {:?}", worker_id, s_q);

                let res_msg = QueryWorker::handle_select(worker_id, s_q).await;
                self.send_msg_to_db(res_msg);
            },
            QueryData::CopyQ(c_q) => {
                debug!("Worker '{}' got COPY QUERY: {:?}", worker_id, c_q);

                let query_id = c_q.query_id();
                let table_id = c_q.table_id();
                let table_name = c_q.table_metadata().table_name().to_string();
                let res = QueryWorker::handle_copy(c_q).await;
                let res_msg = QueryWorker::create_copy_query_res_msg(
                    res,
                    worker_id,
                    query_id,
                    table_id,
                    &table_name
                );

                self.send_msg_to_db(res_msg);
            },
        }
    }

    fn create_copy_query_res_msg(
        res: Result<(Vec<(String, u16)>, i32), DbError>,
        worker_id: usize,
        query_id: Uuid,
        table_id: Uuid,
        table_name: &str,
    ) -> DbWorkerMsg
    {
        match res
        {
            Ok((final_col_file_ids, n_rows)) => {
                return 
                    DbWorkerMsg::QueryCompleted(
                        worker_id,
                        QueryCompletionMsg::new(
                            query_id, 
                            table_id, 
                            n_rows, 
                            WorkerMsgRes::CopyRes(final_col_file_ids)
                ));
            },
            Err(db_err) => {
                return QueryWorker::msg_from_db_err(
                    db_err, 
                    worker_id, 
                    query_id, 
                    table_id, 
                    table_name, 
                    COPY_QUERY_NAME
                );
            }
        }
    }

    async fn handle_copy(
        c_q: CopyQData
    ) -> Result<(Vec<(String, u16)>, i32), DbError>
    {
        QueryWorker::validate_copy_query(
            c_q.query_data(), 
            c_q.table_metadata()
        )?;

        let table_meta = c_q.table_metadata();
        let col_name_to_idx_map = table_meta.get_column_name_to_index_map();
        let query_data = c_q.query_data();
        let src_file_path = query_data.src_filepath();
        let f_csv = QueryWorker::open_csv_file(
            src_file_path, 
            table_meta.table_name()
        ).await?;

        // At this point we know that in copy query dest_columns if exist are 
        // ok and the same as in table, csv file can be opened and table name 
        // in copy query is correct.
        // So now we need to check if in real csv everything is correct
        let mut csv_rdr = QueryWorker::create_csv_reader(query_data, f_csv);
            
        QueryWorker::validate_csv_header(
            query_data,
            table_meta,
            &col_name_to_idx_map,
            &mut csv_rdr
        ).await?;

        let (final_col_file_ids, row_count) = QueryWorker::load_csv_data(query_data, table_meta, &mut csv_rdr).await?;

        return Ok((final_col_file_ids, row_count));
    }

    async fn load_csv_data(
        query_data: &CopyQuery, 
        table_meta: &TableMetadata,
        csv_rdr: &mut csv_async::AsyncReader<tokio::fs::File>
    ) -> Result<(Vec<(String, u16)>, i32), DbError>
    {
        // TODO: rework this function, its too big and convoluted
        // col_data_vec needed for saving BATCH chunks of data to our file 
        // format
        let dest_cols = query_data.dest_columns();
        let mut col_data_vec = table_meta.create_col_data_vec()?;
        let mut batches: Vec<DataColumn> = 
            QueryWorker::init_batches_for_columns(&col_data_vec); 
        let mut record = csv_async::StringRecord::new();
        let mut row_count: i32 = 0;

        while csv_rdr.read_record(&mut record).await?
        {
            row_count = match row_count.checked_add(1) {
                Some(val) => val,
                None => {
                    return Err(
                        DbError::SizeExceeded { 
                            msg: format!("While doing copy query, we read  too many rows from csv for table: {}", table_meta.table_name()), 
                            max: i32::MAX as usize
                        }
                    )
                }
            };

            let csv_vals_vec: Vec<&str> = record.iter().collect();
            match dest_cols
            {
                None => {
                    if csv_vals_vec.len() != batches.len()
                    {
                        return Err(
                            DbError::CsvError(
                                format!(
                                    "We read a row from csv (no dest_cols set) that has different nbr of columns ({}) than our table schema ({})", csv_vals_vec.len(), batches.len()
                                )
                            )
                        )
                    }

                    QueryWorker::push_values_to_batches(
                        &csv_vals_vec,
                        &mut batches,
                        row_count,
                        table_meta.table_name()
                    )?;
                },
                Some(dest_cols) => {
                    if csv_vals_vec.len() < batches.len()
                    {
                        return Err(
                            DbError::CsvError(
                                format!(
                                    "We read a row from csv (dest_cols set) that has less columns ({}) than our table schema ({})", csv_vals_vec.len(), batches.len()
                                )
                            )
                        )
                    }
                    // if csv_vals_vec.len() != dest_cols.len()
                    // {
                    //     return Err(
                    //         DbError::CsvError(
                    //             format!(
                    //                 "We read a row from csv (dest_cols set) that has different nbr of columns ({}) than our dest cols ({})", csv_vals_vec.len(), dest_cols.len()
                    //             )
                    //         )
                    //     )
                    // }

                    // Since we have dest_cols, we will do mapping so that at
                    // 0 position of mapped_csv_vals we have a value for column
                    // that is also at 0 position in our table
                    let mapped_csv_vals = QueryWorker::map_csv_row_to_table_order(
                        dest_cols, 
                        col_data_vec.len(), 
                        &table_meta.get_column_name_to_index_map(),
                        &csv_vals_vec
                    )?;

                    QueryWorker::push_values_to_batches(
                        &mapped_csv_vals,
                        &mut batches,
                        row_count,
                        table_meta.table_name()
                    )?;
                }
            }

            if row_count as usize % BATCH_SIZE == 0
            {
                QueryWorker::save_batches_to_files(
                    &batches, 
                    &mut col_data_vec
                ).await?;

                // we need to remember to clear our batch vectors, since otherwise we will save many times the same data and eventually get error when we exceed BATCH_SIZE
                for batch in &mut batches
                {
                    batch.clear_batch();
                }

            }
        }

        if row_count == 0
        {
            return Err(DbError::CsvError(
                format!("Loading csv for table: {}, provided csv has 0 rows, we do not accept such csvs in COPY query, provide some data", table_meta.table_name())
            ));
        }
        // Last batch might not be equal to BATCH_SIZE thus we need to check it
        // and if it isnt we need to save it since loop didnt do it
        if row_count as usize % BATCH_SIZE != 0
        {
            QueryWorker::save_batches_to_files(
                &batches, 
                &mut col_data_vec
            ).await?;
        }
        
        // We will need this to update file paths in columns in metadata
        let after_save_col_ids: Vec<(String, u16)> = col_data_vec
            .iter()
            .map(|col_data| {
                match col_data {
                    ColDataWrapper::IntColData(int_col) => {
                        let col_name = int_col.col_name().to_string();
                        let file_id = int_col.col_file_id();
                        (col_name, file_id)
                    },
                    ColDataWrapper::StrColData(str_col) => {
                        let col_name = str_col.col_name().to_string();
                        let file_id = str_col.col_file_id();
                        (col_name, file_id)
                    }
                }
            })
            .collect();

        return Ok((after_save_col_ids, row_count));
    }

    async fn save_batches_to_files(
        batches: &Vec<DataColumn>,
        col_data_vec: &mut Vec<ColDataWrapper>
    ) -> Result<(), DbError>
    {
        for (batch_idx, batch) in batches.iter().enumerate()
        {
            match batch
            {
                DataColumn::Int64(i_batch) => {
                    let col_data = match col_data_vec
                                            .get_mut(batch_idx)
                                            .unwrap() {
                        ColDataWrapper::IntColData(i_col_data) => {
                            i_col_data
                        },
                        _ => {
                            return Err(
                                DbError::InternalDbError(
                                    format!(
                                        "QueryWorker::handle_copy: while saving batch, col_data_vec at idx: '{}' should be i64 but isnt", batch_idx
                                    )
                                )
                            );
                        }
                    };

                    col_data.save_to_file(i_batch.values()).await?;
                },
                DataColumn::Varchar(s_batch) => {
                    let col_data = match col_data_vec
                                            .get_mut(batch_idx)
                                            .unwrap() {
                        ColDataWrapper::StrColData(s_col_data) => {
                            s_col_data
                        },
                        _ => {
                            return Err(
                                DbError::InternalDbError(
                                    format!(
                                        "QueryWorker::handle_copy: saving batch - col_data_vec at idx: '{}' should be Varchar but isnt", batch_idx
                                    )
                                )
                            );
                        }
                    };

                    col_data.save_to_file(s_batch.values()).await?;
                }
            }
        }
        return Ok(());
    }

    fn map_csv_row_to_table_order<'a>(
        dest_cols: &Vec<String>,
        n_table_cols: usize,
        col_name_to_idx_map: &HashMap<String, usize>,
        csv_row: &Vec<&'a str>
    ) -> Result<Vec<&'a str>, DbError>
    {
        // dest_cols might have greater length than nbr of table columns
        // but we will take only first columns().len values
        // if csv_row.len() != dest_cols.len() || dest_cols.len() < n_table_cols
        if dest_cols.len() < n_table_cols
        {
            return Err(DbError::InternalDbError(
                format!("QueryWorker::map_csv_row_to_table_order - csv_row.len != dest_cols.len or dest_cols < n_table_cols - at this point this should never happen since these things should've been checked earlier - DB in corrupted state")
            ));
        }

        let mut res_vec: Vec<&str> = vec![""; n_table_cols];
        let csv_row = &csv_row[..n_table_cols];
        for (idx, csv_val) in csv_row.iter().enumerate()
        {
            let dest_name = dest_cols.get(idx).unwrap();
            let name_idx = col_name_to_idx_map.get(dest_name).unwrap();

            *res_vec.get_mut(*name_idx).unwrap() = *csv_val;
        }

        return Ok(res_vec);
    }

    fn push_values_to_batches(
        vals_vec: &Vec<&str>,
        batches: &mut Vec<DataColumn>,
        row_idx: i32,
        table_name: &str
    ) -> Result<(), DbError>
    {
        for (idx, val) in vals_vec.iter().enumerate()
        {
            let batch = batches.get_mut(idx).unwrap();
            match batch
            {
                DataColumn::Int64(i_vec) => {
                    let parsed_val: i64 = match val.parse() {
                        Ok(parsed) => parsed,
                        Err(e) => {
                            return Err(
                                DbError::CsvError(
                                    format!(
                                        "At row: '{}', failed to parse value: '{}' into i64,
                                        for table: '{}', column: '{}'. #ERROR#: {}", row_idx, val, table_name, idx, e
                                    )
                                )
                            );
                        }
                    };
                    i_vec.push(parsed_val);
                },
                DataColumn::Varchar(s_vec) => {
                    // val is alread a string
                    s_vec.push(*val);
                }
            }
        }
        return Ok(());
    }

    fn init_batches_for_columns(
        col_data_vec: &Vec<ColDataWrapper>
    ) -> Vec<DataColumn>
    {
        let mut batches: Vec<DataColumn> = vec![]; 

        for col in col_data_vec
        {
            match col
            {
                ColDataWrapper::IntColData(_) => {
                    batches.push(DataColumn::Int64(
                        Int64Column::new(vec![])
                    ));
                },
                ColDataWrapper::StrColData(_) => {
                    batches.push(DataColumn::Varchar(
                        VarcharColumn::new(vec![])
                    ));
                }
            }
        }

        return batches;
    }

    fn create_csv_reader(
        query_data: &CopyQuery, 
        f_csv: tokio::fs::File
    ) -> csv_async::AsyncReader<tokio::fs::File>
    {
        if query_data.csv_contains_header() 
        {
            return csv_async::AsyncReaderBuilder::new()
                .has_headers(true)
                .delimiter(CSV_DELIM)
                .create_reader(f_csv);
        } 
        else 
        {
            return csv_async::AsyncReaderBuilder::new()
                .has_headers(false)
                .delimiter(CSV_DELIM)
                .create_reader(f_csv)
        }
    }

    async fn open_csv_file(
        src_file_path: &str, 
        table_name: &str
    ) -> Result<tokio::fs::File, DbError>
    {
        match tokio::fs::File
            ::open(src_file_path).await {
                Ok(f) => {
                    return Ok(f);
                },
                Err(_) => {
                    return Err(DbError::NotFound(format!("File path: '{}' to csv for table: '{}' couldn't be opened", 
                    src_file_path, table_name)));
                }
        };

    }

    async fn validate_csv_header(
        query_data: &CopyQuery, 
        table_meta: &TableMetadata,
        col_name_to_idx_map: &HashMap<String, usize>,
        csv_rdr: &mut csv_async::AsyncReader<tokio::fs::File>
    ) -> Result<(), DbError>
    {
        if query_data.csv_contains_header()
        {
            let headers = match csv_rdr.headers().await {
                Ok(val) => val,
                Err(e) => {
                    return Err(DbError::Other(format!(
                        "Malformed csv, error: {}", e
                    )));
                }
            };

            match query_data.dest_columns()
            {
                None => {

                    let columns = table_meta.columns();
                    // If there are no destination columns, we expect to have a csv
                    // with exact number of columns as in our table and these 
                    // columns need to have same ordering and names
                    if headers.len() != columns.len()
                    {
                        return Err(
                            DbError::WrongSize(
                                format!(
                                    "In Copy query for table: '{}', no destination columns specified, csv hase different nbr of columns ({}) than table schema ({})", 
                                    table_meta.table_name(), headers.len(), columns.len()
                                )
                            )
                        )
                    }
                    for (idx, col_name) in headers.iter().enumerate()
                    {
                        // we can unwrap since above we checked if the same lens
                        let col_name_in_table = columns.get(idx).unwrap().c_name();
                        if !col_name_to_idx_map.contains_key(col_name)
                        {
                            return Err(
                                DbError::NotFound(
                                    format!(
                                        "In COPY query for table: {}, without destination columns specified, in csv there is column with name: '{}' that doesn't exist in table schema", table_meta.table_name(), col_name
                                    )
                                )
                            );
                        }
                        if col_name != col_name_in_table
                        {
                            return Err(
                                DbError::NotFound(
                                    format!(
                                        "In COPY query for table: {}, without destination columns specified, in csv: {} column has name: '{}' which is different from column in table schema at the same position: '{}' ", 
                                        table_meta.table_name(), idx, col_name,
                                        col_name_in_table
                                    )
                                )
                            );
                        }
                    }
                },
                Some(dest_cols) => {
                    let columns = table_meta.columns();
                    // If there are destination columns, we expect to have at 
                    // least as many columns in csv as in our db schema. 
                    // Because otherwise mapping cannot be done since we do not
                    // allow NULL values, also dest_cols and header must have 
                    // the same length.
                    // IF headers.len > columns.len we will use only columns.len
                    // from dest_cols
                    if headers.len() < columns.len() 
                    // || headers.len() != dest_cols.len() // ??
                    {
                        return Err(
                            DbError::WrongSize(
                                format!(
                                    "In Copy query for table: '{}', with destination columns specified, csv has different nbr of columns ({}) than parameter dest_columns ({})", 
                                    table_meta.table_name(), headers.len(), dest_cols.len()
                                )
                            )
                        )
                    }
                }
            }
        }
        return Ok(());
    }

    fn validate_copy_query(
        copy_q: &CopyQuery, 
        table_meta: &TableMetadata
    ) -> Result<(), DbError>
    {
        if copy_q.dest_table_name() != table_meta.table_name()
        {
            return Err(
                DbError::Other(
                    format!(
                        "In copy query we have table name: '{}', however in table metadata we have: '{}'", copy_q.table_name(), table_meta.table_name()
                    )
                )
            );
        }

        if let Some(dest_columns) = copy_q.dest_columns()
        {
            let table_col_names: HashSet<&str> = table_meta
                                    .columns()
                                    .iter()
                                    .map(|col| col.c_name())
                                    .collect();

            if dest_columns.len() < table_col_names.len()
            {
                return Err(
                    DbError::SizeMismatch {
                        msg: format!(
                            "Number of destination columns ({}) is less than a number of table columns ({}), thus not all columns can be matched, and we do not allow NULL values",
                            dest_columns.len(),
                            table_col_names.len()
                        ),
                        size_1: dest_columns.len(),
                        size_2: table_col_names.len()
                });
            }

            // Check if all dest column names exist in our table
            for col_name in dest_columns
            {
                if !table_col_names.contains(&col_name.as_str())
                {
                    return Err(DbError::NotFound(
                        format!("Dest Column '{}' not found in table '{}'.\nAvailable columns in this table are: {:?}", 
                        col_name, 
                        table_meta.table_name(),
                        table_col_names
                    )
                ));
                }
            }

            // Check for duplicate column names in destination columns
            let mut seen_cols = std::collections::HashSet::new();
            for col_name in dest_columns {
                if !seen_cols.insert(col_name) {
                    return Err(DbError::Other(
                        format!("Duplicate column name '{}' in destinationColumns for table: '{}'", col_name, table_meta.table_name())
                    ));
                }
            }

        }
        return Ok(());
    }

    async fn handle_select(
        worker_id: usize,
        s_q: SelectQData
    ) -> DbWorkerMsg 
    {
        let table_meta: &TableMetadata = s_q.table_metadata();
        // TODO: move this to other function
        match table_meta.all_column_files_are_empty()
        {
            Ok(is_empty) => {
                if is_empty
                {
                    let n_rows = 0;
                    return DbWorkerMsg::QueryCompleted(
                        worker_id, 
                        QueryCompletionMsg::new(
                            s_q.query_id(), 
                            s_q.table_id(), 
                            n_rows, 
                            WorkerMsgRes::SelectRes(
                                QueryResult::new(n_rows as usize, vec![])
                            )
                        )
                    )
                };
            },
            Err(e) => {
                return DbWorkerMsg::InternalError(
                    worker_id, 
                    QueryFailureMsg::new(
                        s_q.query_id(), 
                        s_q.table_id(), MultipleProblemsError::new_with_one_problem(
                            &e.to_string(), 
                            "QueryWorker::handle_select"
                        )
                    )
                );
            }
        }

        let query_id = s_q.query_id();
        let table_id = table_meta.table_id();
        let table_name = table_meta.table_name();
        let query_result = table_meta.read_table().await;

        match query_result
        {
            Ok((q_res, n_rows)) => {
                debug!("In worker after handling select query: n_rows: {}, \nquery res: {:?}", n_rows, q_res);
                return 
                    DbWorkerMsg::QueryCompleted(
                        worker_id,
                        QueryCompletionMsg::new(
                            query_id, 
                            table_id, 
                            n_rows, 
                            WorkerMsgRes::SelectRes(q_res)
                ));
            },
            Err(db_err) => {
                return QueryWorker::msg_from_db_err(
                    db_err, 
                    worker_id, 
                    query_id, 
                    table_id, 
                    table_name, 
                    "SELECT",
                );
            }
        }
    }

    fn send_msg_to_db(&self, msg: DbWorkerMsg)
    {
        // If we cannot send msg to db it means channel was
        // closed so we should panic and end execution
        // We could use HealthChecks Algorithm from Distributed Systems
        self.tx_to_db.send(DbCmd::DbWorker(msg)).unwrap();
    }

    fn msg_from_db_err(
        db_err: DbError,
        worker_id: usize,
        query_id: Uuid,
        table_id: Uuid,
        table_name: &str,
        query_name: &str,
    ) -> DbWorkerMsg
    {
        match db_err
        {
            DbError::InternalDbError(e) => {
                return DbWorkerMsg::InternalError(
                    worker_id,
                    QueryFailureMsg::new(
                        query_id, 
                        table_id, 
                        MultipleProblemsError::new_with_one_problem(
                            &e,
                            &format!("QueryWorker::{}:: When reading table: '{}' we got error", query_name, table_name)
                    )
                ));
            },
            // We treat IOErrors as internalDbErrors and want to 
            // shutdown db, since here we are handling SELECT so only 
            // reading data from db, so this means that DbMetadata has
            // info about given table, but we couldnt read it (i.e. 
            // somebody removed files, or corrupted them).
            // Thus our whole db is in corrupted state and we want to 
            // end it's execution
            DbError::IoError(e) => {
                return DbWorkerMsg::InternalError(
                    worker_id,
                    QueryFailureMsg::new(
                        query_id, 
                        table_id, 
                        MultipleProblemsError::new_with_one_problem(
                            &e.to_string(),
                            &format!("QueryWorker::{}:: When reading table: '{}' we got IO error", query_name, table_name)
                    )
                ));
            },
            _ => {
                return DbWorkerMsg::QueryFailed(
                    worker_id,
                    QueryFailureMsg::new(
                    query_id, 
                    table_id, 
                    MultipleProblemsError::new_with_one_problem(
                        &db_err.to_string(),
                        &format!("QueryWorker::handle_select:: When reading table: '{}' we got error", table_name)
                    )
                ));
            }
        }
    }

}