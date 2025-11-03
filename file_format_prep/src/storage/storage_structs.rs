pub const CELL_DATA_SIZE: usize = 250;
pub const MAGIC_WORD_1: u32 = 0xF1FAA;


#[derive(Debug, Clone, Copy)]
pub struct CellHeader
{
    is_overflow: bool,
    overflow_ptr: u16,
    size_of_data: u8,
    next_cell_offset: u16, // if 0 we have new page
}

impl CellHeader
{
    pub fn new() -> CellHeader
    {
        CellHeader {is_overflow: false, overflow_ptr: 0, size_of_data: 0, next_cell_offset: 0 }
    }
}

pub struct Cell
{
    /// In total Cell takes 256 bytes
    /// First Cell of each column stores column name
    cell_header: CellHeader,
    data: [u8; CELL_DATA_SIZE]
}

impl Cell 
{
    pub fn new(h: CellHeader) -> Cell
    {
        Cell {cell_header: h, data: [0; CELL_DATA_SIZE]}
    }

    pub fn insert_data(
        &mut self, 
        byte_arr: &[u8], 
        first_free_idx: usize
    ) -> usize
    {
        let mut i = first_free_idx;

        for byte in byte_arr
        {
            *self.data.get_mut(i).unwrap() = *byte;
            i += 1;
        }

        i
    }
}

pub struct ColRow
{
    /// In row we store column's data
    /// 
    /// 
    idx: u32,           // curr row idx, starts from 1, 
    next_row: u32,      // offset to the next row
    row_size: u16,      // size of row's data (vec of cells)
    cells: Vec<Cell>    // 
}

pub struct StorageFormat
{
    magic_word: u32,
    col_types: Vec<bool>, // if true column is String
    offsets_to_rows: Vec<u32>,
    colrows: Vec<ColRow>
}

impl StorageFormat 
{
    pub fn new(
        magic_word: u32, 
        col_types: Vec<bool>,
        offsets_to_rows: Vec<u32>,
        colrows: Vec<ColRow>
        ) -> StorageFormat
    {
        StorageFormat { magic_word, col_types, offsets_to_rows, colrows }
    }
}