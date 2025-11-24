use std::fs::File;

pub fn read_csv(file_path: &str, delim: u8) 
    -> (Vec<u8>, Vec<String> , Vec<Vec<String>>)
{
    // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
    // !!!!!!!!!!!! CURRENTLY THIS IS REALLY SIMPLE CSV READER !!!!!!!!!!!!!!!!
    // !!!!! so that we have a way to easily populate our files with data !!!!!
    // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
    // 
    // CSV file format
    // -- first row: types of column, either 's' or 'i'
    // -- second row: column names, max 255 characters, 
    //                satisfying regex: ^[a-zA-Z][a-zA-Z0-9_]*$

    let file = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => panic!("Error: {e}")
    };


    // TODO: check if csv:: uses streams
    let mut rdr = csv::ReaderBuilder::new()
                        .has_headers(true)
                        .delimiter(delim)
                        .from_reader(file);


    // first row needs to have types: 's' or 'i', thus they are headers
    let headers = rdr.headers().unwrap().clone();
    let types_vec = Vec::from_iter(headers.iter());
    let types_vec: Vec<u8> = types_vec.iter().map(|x| {
            if *x == "s" {1}
            else if *x == "i" {0}
            else{2} // unsuppoorted type
        })
        .collect();

    let mut columns: Vec<Vec<String>> = Vec::new();
    let mut col_names: Vec<String> = Vec::new();

    let n_cols = types_vec.len();

    // Creating 2D vector, columns[0][..] are values of first column
    for _ in 0..n_cols
    {
        columns.push(Vec::new());
    }

    let mut is_col_names = true;

    for result in rdr.records()
    {
        // col names are in first row
        if is_col_names
        {
            col_names = result
                            .unwrap()
                            .iter()
                            .map(|x| x.to_string())
                            .collect();
            is_col_names = false;
        }
        else 
        {
            let record = match result {
                Ok(r) => r,
                Err(e) => panic!("Error: for loop else branch in read_csv: {e}")
            };

            for (idx, col_val) in record.iter().enumerate()
            {
                columns
                    .get_mut(idx % n_cols)
                    .unwrap()
                    .push(String::from(col_val));
            }
        }
    }
    (types_vec, col_names, columns)
}