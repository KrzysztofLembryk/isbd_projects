use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::vec;

use std::io::Error as io_err;
use std::io::ErrorKind as io_errkind;

pub const MAGIC_WORD: u32 = 0xF1FAA;
const METADATA_INIT_STAGE_SIZE: usize = 6;
// 2^16 bytes buffer, we don't expect metadata file to be big
const BUFF_SIZE: usize = 65535;

#[derive(PartialEq)]
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
    ///             reading next bytes we know how many 
    /// -- col_names: every string will end with '\0',
    ///               TODO:
    ///               regex for col name: ^[a-zA-Z][a-zA-z0-9_]*$, max len: 255
    /// -- col_types: 1 if string, 0 if int
    /// -- col_files_count: these numbers say how many files one column takes
    /// -- col_files_paths: paths to given files
    magic_word: u32,
    col_count: u16,
    col_files_count: Vec<u8>,
    col_types: Vec<u8>,
    col_names: Vec<String>, 
    col_files_paths: HashMap<String, Vec<String>>, // k: col_name, v: file_path
}

impl  DbMetadata  {
    pub fn new(
        col_count: u16, 
        col_files_count: Vec<u8>,
        col_types: Vec<u8>,
        col_names: Vec<String>,
        col_files_paths: HashMap<String, Vec<String>>
    ) -> DbMetadata
    {
        DbMetadata { 
            magic_word: MAGIC_WORD, 
            col_count, 
            col_files_count, 
            col_types, 
            col_names, 
            col_files_paths
        }
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), io_err>
    {
        let mut f = File::create(path)?;
        let null_terminator = [b'\0'];

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
            if !col_name.is_ascii()
            {
                return Err(io_err_wrapper(&format!("Column: '{}' is not ASCII", col_name)));
            }
            f.write(col_name.as_bytes())?;
            f.write(&null_terminator)?;
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

                if !file_path.is_ascii()
                {
                    return Err(io_err_wrapper(&format!("File_path: '{}' is not ASCII", col_name)));
                }
                f.write(file_path.as_bytes())?;
                f.write(&null_terminator)?;
            }
        }

        f.flush()?;

        Ok(())
    }
    
    pub fn read_from_file(path: &str) -> Result<DbMetadata, io_err>
    {
        let mut f = File::open(path)?;
        
        let mut buf = [0 as u8; BUFF_SIZE]; 
        let mut curr_buf_idx: usize = 0;
        let mut progress_idx: usize = 0; 
        
        let mut magic_word: u32 = 0;
        let mut col_count: u16 = 0;
        let mut col_files_count: Vec<u8> = vec![];
        let mut col_types: Vec<u8> = vec![];
        let mut col_names: Vec<String> = vec![];
        let mut col_files_paths: HashMap<String, Vec<String>> = HashMap::new();
        let mut curr_stage = ReadStages::InitStage;

        loop 
        {
            let bytes_read = f.read(&mut buf)?;
            curr_buf_idx = 0;
            
            if bytes_read == 0
            {
                if curr_stage != ReadStages::EndedReading
                {
                    return Err(io_err_wrapper("There was to little data in file"));
                }

                println!("DbMetadata - read_from_file - SUCCESS, Read 0 bytes, breaking");
                break;
            }

            if curr_stage == ReadStages::InitStage
            {
                curr_stage = read_metadata_init_stage(bytes_read, 
                                                            &buf, 
                                                            &mut magic_word, 
                                                            &mut col_count)?;
                curr_buf_idx = 6;

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
                }
            }

            if curr_stage == ReadStages::FilePathStage
            {

            }
        }

        Ok(DbMetadata::new(
            col_count, 
            col_files_count, 
            col_types, 
            col_names, 
            col_files_paths))
    }
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
    let mut eos_present = false;
    let col_count = col_count as usize;

    while *progress_idx < col_count 
    {
        let col_name = res.get_mut(*progress_idx).unwrap();

        loop
        {
            if *curr_buf_idx >= bytes_read
            {
                break;
            }

            let c = buf[*curr_buf_idx];

            if c == b'\0'
            {
                eos_present = true;
                break;
            }

            col_name.push(c as char);
            *curr_buf_idx += 1;
        }

        if !eos_present
        {
            // means that curr_buf_idx >= bytes_read and we need
            // to read another portion of bytes into buf
            return Ok(stage);
        }

        eos_present = false;
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
        Err(io_err_wrapper("DbMetadata - read_metadata_str_stage got unsopported stage"))
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

        res.push(buf[*curr_buf_idx]);
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
        Err(io_err_wrapper("read_metadata_u8_stage got unsopported stage"))
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
            io_err_wrapper("Metadata file has to little data - Init stage")
        );
    }

    *magic_word = u32::from_be_bytes(
                    buf[..4]
                    .try_into()
                    .expect("DbMetadata - read_from_file - magic word from buff transformation error"));

    if *magic_word != MAGIC_WORD
    {
        return Err(
            io_err_wrapper("Magic word at the begginnig of metadata file is incorrect")
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

fn io_err_wrapper(msg: &str) -> io_err
{
    io_err::new(io_errkind::Other, msg)
}
