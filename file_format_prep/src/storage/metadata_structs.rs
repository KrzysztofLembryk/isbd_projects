use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::vec;
use std::fmt;
use regex::Regex;

use std::io::Error as io_err;
use crate::errors::io_other_err_wrapper;
use crate::constants::{MAGIC_WORD, DB_DATA_DIR};
use crate::storage::string_handlers::{StrLenCheckType, 
    read_string_from_buf, save_string_to_file_with_null_char};

//##############################################################################
//############################# CONSTANTS ######################################
//##############################################################################

const METADATA_INIT_STAGE_SIZE: usize = 6;
const AFTER_INIT_STAGE_BUFF_IDX: usize = 6;
// 2^16 bytes buffer, we don't expect metadata file to be big
const BUFF_SIZE: usize = 65535;

// const BUFF_SIZE: usize = 6;

#[derive(PartialEq, Debug)]
enum ReadStages {
    InitStage,          // this is initial stage where we always read 6 bytes
    FilesCountStage,    // this is u8 stage 
    ColTypesStage,      // this is u8 stage too; values we read are only u8
    ColNameStage,       // these are String stages, and we read strings of 
    FilePathStage,       // any length
    EndedReading
}

pub struct DbMetadata
{
    /// In DbMetadata struct we will store all file names, sizes, dirs, etc 
    /// -- magic_word: word at the beginning of file saying that this is file
    ///                of our database
    /// -- col_count: we will store how many columns we have, so that when 
    /// -- col_names: every string will end with '\0',
    /// -- col_types: 1 if string, 0 if int
    /// -- col_files_count: these numbers say how many files one column has
    /// -- col_files_paths: paths to given files
    magic_word: u32,
    col_count: u16,
    col_files_count: Vec<u8>,
    col_types: Vec<u8>, // TODO: change this to AllowedTypes
    col_names: Vec<String>, 
    col_files_paths: HashMap<String, Vec<String>>, // k: col_name, v: file_path
    col_names_idxs: HashMap<String, usize>
}

impl  DbMetadata  {
    pub fn new_all_data(
        col_count: u16, 
        col_files_count: Vec<u8>,
        col_types: Vec<u8>,
        col_names: Vec<String>,
        col_files_paths: HashMap<String, Vec<String>>
    ) -> Result<DbMetadata, io_err>
    {
        check_metadata_correctness(
            col_count as usize, 
            &col_files_count, 
            &col_types, 
            &col_names, 
            &col_files_paths)?;

        // We will need this for quick lookup of col names idx when we have 
        // only column name
        let mut idxs = HashMap::new();
        for (id, name) in col_names.iter().enumerate()
        {
            idxs.insert(name.clone(), id);
        }

        Ok(DbMetadata { 
            magic_word: MAGIC_WORD, 
            col_count, 
            col_files_count, 
            col_types, 
            col_names, 
            col_files_paths,
            col_names_idxs: idxs            
        })
    }

    pub fn new(
        col_types: Vec<u8>,
        col_names: Vec<String>
    ) -> Result<DbMetadata, io_err>
    {
        if col_types.len() != col_names.len()
        {
            return Err(io_other_err_wrapper("DbMetadata - new_basic - col_types vec has diff len than col_names"));
        }
        if col_types.len() > u16::MAX as usize
        {
            return Err(io_other_err_wrapper("DbMetadata - new_basic - number of columns is greater than u16::MAX"));
        }

        let col_count = col_types.len();
        let col_files_count: Vec<u8> = vec![1; col_count];
        let mut col_files_paths: HashMap<String, Vec<String>> = HashMap::new();

        // Creating simple file paths
        for name in &col_names
        {
            // At first we have only one file for each column, when we will read
            // enough data, we will create another one if the first is full
            let file_path = format!("{DB_DATA_DIR}/{name}_0");
            col_files_paths.insert(name.clone(), vec![file_path]);
        }

        // TODO: probably we should use here DbMetadata::new_all_data
        // instead of repeating code
        DbMetadata::new_all_data(
            col_count as u16, 
            col_files_count, 
            col_types, 
            col_names, 
            col_files_paths)
    }

    pub fn new_empty() -> Result<DbMetadata, io_err>
    {
        DbMetadata::new(Vec::new(), Vec::new())
    }

    /// We do not encode metadata when saving to file
    pub fn save_to_file(&self, path: &str) -> Result<(), io_err>
    {
        let mut f = File::create(path)?;
        // let null_terminator = [b'\0'];

        f.write(&self.magic_word.to_be_bytes())?;

        // We save col_count so we know how many columns there is thus we will
        // also know how many col_files_count and col_types there are
        f.write(&self.col_count.to_be_bytes())?;
        f.write(&self.col_files_count[..])?;
        f.write(&self.col_types[..])?;

        // When saving strings to files we need to add null termination '\0'
        // to the end of string, since rust uses pointer+length encoding
        for col_name in &self.col_names
        {
            // if !col_name.is_ascii()
            // {
            //     return Err(io_other_err_wrapper(&format!("Column: '{}' is not ASCII", col_name)));
            // }
            // f.write(col_name.as_bytes())?;
            // f.write(&null_terminator)?;
            save_string_to_file_with_null_char(col_name, &mut f)?;
        }

        // We want to maintain the same order for files paths as we have for 
        // column names, so that we know that first n files paths belongs to first column etc. 
        for col_name in &self.col_names
        {
            let col_paths = self
                .col_files_paths
                .get(col_name)
                .expect("DbMetadata - save_to_file - col_files_paths.get got Error - no such col_name in hashmap");

            for file_path in col_paths
            {
                // if !file_path.is_ascii()
                // {
                //     return Err(io_other_err_wrapper(&format!("File_path: '{}' is not ASCII", col_name)));
                // }
                // f.write(file_path.as_bytes())?;
                // f.write(&null_terminator)?;
                save_string_to_file_with_null_char(&file_path, &mut f)?;
            }
        }

        f.flush()?;

        Ok(())
    }
    
    pub fn read_from_file(path: &str) -> Result<DbMetadata, io_err>
    {
        let mut f = File::open(path)?;
        
        let mut buf = [0 as u8; BUFF_SIZE]; 
        let mut curr_buf_idx: usize;
        let mut progress_idx: usize = 0; 
        
        let mut magic_word: u32 = 0;
        let mut col_count: u16 = 0;
        let mut col_files_count: Vec<u8> = vec![];
        let mut col_types: Vec<u8> = vec![];
        let mut col_names: Vec<String> = vec![];
        let mut col_files_paths: HashMap<String, Vec<String>> = HashMap::new();
        let mut curr_stage = ReadStages::InitStage;
        let mut curr_col_idx: usize = 0;

        // TODO: Add LOGGING
        loop 
        {
            let bytes_read = f.read(&mut buf)?;
            curr_buf_idx = 0;
            
            match check_if_break_loop(bytes_read, &curr_stage)
            {
                Ok(x) => if x {break;},
                Err(e) => return Err(e)
            }

            if curr_stage == ReadStages::InitStage
            {
                curr_stage = read_metadata_init_stage(bytes_read, 
                                                            &buf, 
                                                            &mut magic_word, 
                                                            &mut col_count)?;
                // in init stage we always read 6 bytes, thus we need to set 
                // curr_buf_idx to 6
                curr_buf_idx = AFTER_INIT_STAGE_BUFF_IDX;

                if col_count == 0 && bytes_read == METADATA_INIT_STAGE_SIZE
                {
                    // No columns means empty db so we just return it without
                    // doing next steps
                    return DbMetadata::new_all_data(
                                                    col_count, 
                                                    col_files_count, 
                                                    col_types, 
                                                    col_names, 
                                                    col_files_paths);
                }
                // We know how many columns we will have, so we can create as
                // many empty strings in our vector that will store col names.
                // Doing this will boost our code a little faster.
                init_col_names(&mut col_names, col_count);
            }

            if curr_stage == ReadStages::FilesCountStage
            {
                curr_stage = read_metadata_u8_stage(
                            bytes_read, 
                            &buf, 
                            col_count, 
                            &mut curr_buf_idx, 
                            &mut progress_idx,
                            &mut col_files_count,
                            ReadStages::FilesCountStage)?;
            }

            if curr_stage == ReadStages::ColTypesStage
            {
                curr_stage = read_metadata_u8_stage(
                            bytes_read, 
                            &buf, 
                            col_count, 
                            &mut curr_buf_idx, 
                            &mut progress_idx,
                            &mut col_types,
                            ReadStages::ColTypesStage)?;
            }
            
            if curr_stage == ReadStages::ColNameStage
            {
                curr_stage = read_metadata_str_stage(
                                                    bytes_read, 
                                                    &buf, 
                                                    col_count, 
                                                    &mut curr_buf_idx, 
                                                    &mut progress_idx, 
                                                    &mut col_names, 
                                                    ReadStages::ColNameStage)?;
                
                if curr_stage == ReadStages::FilePathStage
                {
                    // When we change stage, we can initialize hash map that 
                    // that will store file paths to files that will store 
                    // columns' data
                    init_files_paths(
                        &col_names, 
                        &mut col_files_paths, 
                        &col_files_count
                    );

                    // And we can initialize curr_col_idx
                    curr_col_idx = 0;
                }
            }

            if curr_stage == ReadStages::FilePathStage
            {
                curr_stage = read_file_paths(
                    &mut curr_buf_idx, 
                    &mut progress_idx, 
                    &mut curr_col_idx, 
                    curr_stage, 
                    bytes_read, 
                    col_count as usize, 
                    &buf, 
                    &col_names, 
                    &col_files_count, 
                    &mut col_files_paths)?;
            }
        }

        DbMetadata::new_all_data(
            col_count, 
            col_files_count, 
            col_types, 
            col_names, 
            col_files_paths)
    }

    pub fn append_new_file_path(
        &mut self, 
        col_name: &String, 
        file_path: String
    ) -> Result<(), io_err>
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
            return Err(io_other_err_wrapper(&format!("col_name: {} is not present in db_metadata", col_name)));
        }

        Ok(())
    }

    // ###################################################################### 
    // ############################ GETTERS #################################
    // ###################################################################### 
    pub fn col_count(&self) -> u16
    {
        self.col_count
    }

    pub fn col_files_count(&self) -> &Vec<u8> 
    {
        &self.col_files_count
    }

    pub fn col_types(&self) -> &Vec<u8> 
    {
        &self.col_types
    }

    pub fn col_names(&self) -> &Vec<String> 
    {
        &self.col_names
    }

    pub fn col_files_paths(&self) -> &HashMap<String, Vec<String>> 
    {
        &self.col_files_paths
    }

    pub fn col_names_idxs(&self) -> &HashMap<String, usize>
    {
        &self.col_names_idxs
    }
}

impl fmt::Display for DbMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "DbMetadata:")?;
        writeln!(f, "  magic_word: 0x{:X}", self.magic_word)?;
        writeln!(f, "  col_count: {}", self.col_count)?;
        writeln!(f, "  col_files_count: {:?}", self.col_files_count)?;
        writeln!(f, "  col_types: {:?}", self.col_types)?;
        writeln!(f, "  col_names: {:?}", self.col_names)?;
        writeln!(f, "  col_files_paths:")?;
        for (col, paths) in &self.col_files_paths {
            writeln!(f, "    {}: {:?}", col, paths)?;
        }
        Ok(())
    }
}


//##############################################################################
//######################### PRIVATE HELPER FUNCTIONS ###########################
//##############################################################################
fn read_file_paths(
    curr_buf_idx: &mut usize,
    progress_idx: &mut usize,
    curr_col_idx: &mut usize,
    curr_stage: ReadStages,
    bytes_read: usize,
    col_count: usize,
    buf: &[u8],
    col_names: &Vec<String>,
    col_files_count: &Vec<u8>,
    col_files_paths: &mut HashMap<String, Vec<String>>
) -> Result<ReadStages, io_err>
{
    let mut stage = curr_stage;
    loop 
    {
        // curr_col_idx - all column data (i.e. col names, file paths) are 
        // stored in order, so at 0 idx we have col_name_0 and file paths for 
        // this column are stored first in our metadata file thus we want to 
        // read them to file_paths_hashmap at col_name_0 key
        let col_name = col_names
                            .get(*curr_col_idx)
                            .unwrap();
        // Each column has at least 1 file, but nbr of files for each column
        // may differ
        let file_count = *col_files_count
                                .get(*curr_col_idx)
                                .unwrap() as u16;
        // For current column we will store here all of it's file paths strings
        let file_paths_vec = col_files_paths
                                .get_mut(col_name)
                                .unwrap();

        // We will read file_count strings into file_paths_vec here 
        stage = read_metadata_str_stage(
            bytes_read, 
            buf, 
            file_count, 
            curr_buf_idx, 
            progress_idx, 
            file_paths_vec,
            stage)?;
        
        if stage == ReadStages::EndedReading
        {
            // If curr_stage is EndedReading it means that we have 
            // read all file paths for current column, so we can 
            // start reading file paths for next column because
            // THERE IS STILL SOME DATA IN BUFFER
            *curr_col_idx += 1;

            if *curr_col_idx >= col_count 
            {
                // If curr_col_idx == col_count it means that we 
                // have read all data, so we want to leave this loop
                break;
            }
            // We need to set stage to FilePathStage since 
            // read_metadata_str_stage expects this stage to function correctly,
            // otherwise it will throw error since it does not expect sucb stage
            stage = ReadStages::FilePathStage;
        }
        else // curr_stage == FilePathStage
        {
            // We haven't ended reading all data for this column yet
            // but THERE IS NO MORE DATA IN BUFFER
            // thus we need to break from this loop to get more
            // data from read, but we leave curr_stage as 
            // FilePathStage cause we want to come back here
            break;
        }
    }
    Ok(stage)
}

fn read_metadata_str_stage(
    bytes_read: usize, 
    buf: &[u8], 
    col_count: u16,
    curr_buf_idx: &mut usize,
    progress_idx: &mut usize,
    res: &mut Vec<String>, 
    stage: ReadStages
) -> Result<ReadStages, io_err>
{
    // eos = end of string
    // this variable checks if we encountered null termination of 
    // string for current column, if not, we need to read more data
    // let mut eos_present = false;
    let mut eos_present;
    let col_count = col_count as usize;

    while *progress_idx < col_count 
    {
        // res vector is pre-allocated thus we can get_mut string at progress_id
        // position
        let col_name = res.get_mut(*progress_idx).unwrap();

        eos_present = read_string_from_buf(
            curr_buf_idx, 
            bytes_read, 
            buf, 
            col_name, 
            StrLenCheckType::ColNameLenCheck)?;

        if !eos_present
        {
            // means that curr_buf_idx >= bytes_read and we need
            // to read another portion of bytes into buf
            return Ok(stage);
        }

        // eos_present == true thus we read whole string and can advance to the 
        // next one
        *progress_idx += 1;
    }

    *progress_idx = 0;

    if stage == ReadStages::ColNameStage 
    {
        Ok(ReadStages::FilePathStage)
    }
    else if stage == ReadStages::FilePathStage
    {
        Ok(ReadStages::EndedReading)
    }
    else 
    {
        Err(io_other_err_wrapper("DbMetadata - read_metadata_str_stage got unsopported stage"))
    }
}

fn read_metadata_u8_stage(
    bytes_read: usize, 
    buf: &[u8], 
    col_count: u16,
    curr_buf_idx: &mut usize,
    progress_idx: &mut usize,
    res: &mut Vec<u8>, 
    stage: ReadStages
) -> Result<ReadStages, io_err>
{
    let col_count = col_count as usize;

    while *progress_idx < col_count 
    {
        if *curr_buf_idx >= bytes_read
        {
            // We can have up to u16::MAX columns, thus we may have run out of
            // space in our buffer, thus to read all of them we need to return
            // to main loop, read more data to buffer and come back here again
            return Ok(stage);
        }

        let val = buf[*curr_buf_idx];

        if stage == ReadStages::FilesCountStage && val == 0
        {
            // We can only have positive number of files for given column,
            // if column exist then there must be at least 1 file for it
            return Err(io_other_err_wrapper("File count cannot be 0, if column exists there must be at least 1 file for it"));
        }
        else if stage == ReadStages::ColTypesStage && val > 1
        {
            // 1 means column has String type, 0 i64 type, we do not allow any
            // other types
            return Err(io_other_err_wrapper("Column type can be either 1-String or 0-i64"));
        }

        res.push(val);

        *curr_buf_idx += 1;
        *progress_idx += 1;
    }

    // If we end loop this means that all file counts for given columns were 
    // read thus we can go to next stage, and thus we need to zero progress_idx
    *progress_idx = 0;

    // For both File Count stage and Col Types stage we do exactly the same
    // since they both are Vec<u8>
    if stage == ReadStages::FilesCountStage
    {
        Ok(ReadStages::ColTypesStage)
    }
    else if stage == ReadStages::ColTypesStage
    {
        Ok(ReadStages::ColNameStage)
    }
    else 
    {
        Err(io_other_err_wrapper("read_metadata_u8_stage got unsopported stage"))
    }
}

fn read_metadata_init_stage(
    bytes_read: usize, 
    buf: &[u8], 
    magic_word: &mut u32,
    col_count: &mut u16,
) -> Result<ReadStages, io_err>
{
    // for first two values we know exactly how many bytes they take 
    if bytes_read < METADATA_INIT_STAGE_SIZE
    {
        return Err(
            io_other_err_wrapper("Metadata file has to little data - Init stage")
        );
    }

    *magic_word = u32::from_be_bytes(
                    buf[..4]
                    .try_into()
                    .expect("DbMetadata - read_from_file - magic word from buff transformation error"));

    if *magic_word != MAGIC_WORD
    {
        return Err(
            io_other_err_wrapper("Magic word at the begginnig of metadata file is incorrect")
        );
    }

    *col_count = u16::from_be_bytes(
                buf[4..6]
                .try_into()
                .expect("DbMetadata - read_from_file - col_count read from buff transformation error"));
    
    Ok(ReadStages::FilesCountStage)
}

fn init_files_paths(
    col_names: &Vec<String>, 
    col_files_paths: &mut  HashMap<String, Vec<String>>,
    col_files_count: &Vec<u8>
)
{
    // Once we know names of columns, we can initialize 
    // our hash map that stores all file paths for each column 
    for (id, name) in col_names.iter().enumerate()
    {
        col_files_paths.insert(
            name.clone(), 
            // Here we create a vector of EMPTY strings, number of strings 
            // in each vector equals to previously read file_count for given 
            // column
            vec![
                String::new(); 
                *col_files_count.get(id).unwrap() as usize
                ]
        );
    }
}

fn init_col_names(col_names: &mut Vec<String>, col_count: u16)
{
    for _ in 0..col_count
    {
        // We initialise vector of names of column, so that we can
        // push read chars right into these strings, instead of 
        // making temp_str and then cloning it 
        col_names.push(String::new());
    }
}

fn check_metadata_correctness(
    col_count: usize,
    col_files_count: &Vec<u8>,
    col_types: &Vec<u8>,
    col_names: &Vec<String>, 
    col_files_paths: &HashMap<String, Vec<String>>, 
) -> Result<(), io_err>
{
    // if col_count == 0 {
    //     return Err(io_other_err_wrapper("There must be at least one column (col_count == 0)"));
    // }

    if col_files_count.len() != col_count 
        || col_types.len() != col_count 
        || col_names.len() != col_count 
        || col_files_paths.len() != col_count 
    {
        return Err(io_other_err_wrapper("col_names, col_types, col_files_count and col_files_paths hashmap must have the same length equal to col_count"));
    }

    if col_count == 0
    {
        // if db empty, we checked that other variables are also of 0 length 
        // thus there is no sense in checking following cases
        return Ok(())
    }

    if col_files_count.iter().any(|x| *x == 0) {
        return Err(io_other_err_wrapper("Each column must have at least one file (col_files_count contains 0)"));
    }

    if col_types.iter().any(|x| *x > 1) {
        return Err(io_other_err_wrapper("Column types must be 0 (int) or 1 (string) only (col_types contains value > 1)"));
    }

    if col_names.iter().any(|x| x.len() > 255) {
        return Err(io_other_err_wrapper("Column names must not exceed 255 characters"));
    }

    let mut names_set = HashSet::new();
    for name in col_names
    {
        names_set.insert(name.clone());
    }

    if names_set.len() != col_names.len()
    {
        return Err(io_other_err_wrapper("There are duplicates in column names"));
    }

    // As column names we only allow strings wit a-zA-Z characters 
    // underscores and numbers, max length of String is 255
    let re = Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$").unwrap();

    if col_names.iter().any(|x| !re.is_match(x)) {
        return Err(io_other_err_wrapper("Column names must match regex: ^[a-zA-Z][a-zA-Z0-9_]*$"));
    }

    if !col_names.iter().all(|k| col_files_paths.contains_key(k)) {
        return Err(io_other_err_wrapper("col_files_paths must contain all column names as keys"));
    }

    if col_files_paths.iter().any(|(_, paths)| paths.len() == 0) {
        return Err(io_other_err_wrapper("Each column must have at least one file path (col_files_paths contains empty vector)"));
    }

    let re = Regex::new(r"^[a-zA-Z./][a-zA-Z0-9_./]*$").unwrap();
    if col_files_paths.iter().any(|(_, paths)| paths
                                                .iter()
                                                .any(|x| !re.is_match(x)))
    {
        return Err(io_other_err_wrapper("Each file path must satisfy regex: ^[a-zA-Z./][a-zA-Z0-9_./]*$"));
    }

    Ok(())
}

fn check_if_break_loop(
    bytes_read: usize, 
    curr_stage: &ReadStages
) -> Result<bool, io_err>
{
    if bytes_read == 0
    {
        if *curr_stage != ReadStages::EndedReading
        {
            return Err(io_other_err_wrapper("There was to little data in the file"));
        }

        return Ok(true);
    }
    else if bytes_read > 0 && *curr_stage == ReadStages::EndedReading
    {
        return Err(io_other_err_wrapper("Stage is EndedReading but there is still data to be read in buffer"));
    }

    Ok(false)
}

