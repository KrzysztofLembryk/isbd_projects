
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