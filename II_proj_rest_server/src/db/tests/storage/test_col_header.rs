use self::super::*;

#[test]
fn test_check_col_name_correctness() 
{
    let too_long: String = vec!['a'; 256].iter().collect();
    let not_ascii: String = String::from("Some random ątęxtź");
    let not_allowed_space: String = String::from("file name");
    let not_allowed_hash: String = String::from("file_name#");

    // assert!(check_col_name_correctness(&too_long).is_err());
    // assert!(check_col_name_correctness(&not_ascii).is_err());
    // assert!(check_col_name_correctness(&not_allowed_space).is_err());
    // assert!(check_col_name_correctness(&not_allowed_hash).is_err());
}
