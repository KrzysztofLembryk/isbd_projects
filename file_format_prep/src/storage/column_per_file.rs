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
    FilePathStage       // any length
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
        let mut temp_str: String = String::new();

        loop 
        {
            let bytes_read = f.read(&mut buf)?;
            curr_buf_idx = 0;
            
            if bytes_read == 0
            {
                println!("DbMetadata - read_from_file - Read 0 bytes, breaking");
                break;
            }

            if curr_stage == ReadStages::InitStage
            {
                curr_stage = init_stage_read_metadata(bytes_read, 
                                                            &buf, 
                                                            &mut magic_word, 
                                                            &mut col_count)?;
                curr_buf_idx = 6;
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
                // eos = end of string
                // this variable checks if we encountered null termination of 
                // string for current column, if not, we need to read more data
                let mut eos_present = false;

                while progress_idx < col_count as usize
                {
                    if curr_buf_idx >= bytes_read
                    {
                        continue;
                    }
                    
                    loop
                    {
                        if curr_buf_idx >= bytes_read
                        {
                            break;
                        }

                        let c = buf[curr_buf_idx];

                        if c == b'\0'
                        {
                            eos_present = true;
                            col_names.push(temp_str.clone());
                            temp_str.clear();
                            break;
                        }
                        else 
                        {
                            // temp_str.push(c as char);
                            col_names.get_mut(progress_idx).unwrap().push(c as char);
                        }


                        curr_buf_idx += 1;
                    }
                    if !eos_present
                    {
                        // means that curr_buf_idx >= bytes_read and we need
                        // to read another portion of bytes into buf
                    }
                }
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

fn init_stage_read_metadata(
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

fn io_err_wrapper(msg: &str) -> io_err
{
    io_err::new(io_errkind::Other, msg)
}

pub struct ColHeader
{
    magic_word: u32,    // magic word saying that this is our db file
    col_id: u16,        // we will have probably many files for one column, so 
                        // this is just to make sure we read correct column
    // file_seq_id: u16,// tells us in which file in sequence we are 
    col_type: u8,       // either 'i' or 's'
    is_overflow: bool,  // tells us if there are more files with this col data
    size_of_data: u32,  // size of data without metadata
}

pub struct ColData
{
    h: ColHeader,
    data: Vec<u8>
}