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

// if test create_new_empty and create_next work this means that also 
// ColHeader::new() works since internally they use this function
#[test]
fn test_create_new_empty() 
{
    let ok_name = String::from("ok_col");
    let ok_col = ColHeader::new_empty(
                            AllowedColType::IntType, 
                            ok_name.clone());

    assert!(ok_col.is_ok());

    let ok_col = ok_col.unwrap();

    assert_eq!(ok_col.col_id(), 0);
    assert_eq!(ok_col.col_type(), AllowedColType::IntType);
    assert_eq!(ok_col.is_overflow(), false);
    assert_eq!(ok_col.size_of_data(), 0);
    assert_eq!(*ok_col.col_name(), ok_name);

    let not_ascii: String = String::from("Some random ątęxtź");
    let too_long: String = vec!['a'; 256].iter().collect();

    assert!(ColHeader::new_empty(AllowedColType::IntType, not_ascii).is_err());
    assert!(ColHeader::new_empty(AllowedColType::IntType, too_long).is_err());
}

#[test]
fn test_create_next()
{
    let ok_name = String::from("ok_col");
    let ok_col = ColHeader::new_empty(
                            AllowedColType::IntType, 
                            ok_name.clone());

    assert!(ok_col.is_ok());

    let ok_col = ok_col.unwrap();
    let next_col = ok_col.create_next();

    assert!(next_col.is_ok());

    let next_col = next_col.unwrap();

    assert_eq!(next_col.col_id(), ok_col.col_id() + 1);
    assert_eq!(next_col.col_type(), AllowedColType::IntType);
    assert_eq!(next_col.is_overflow(), false);
    assert_eq!(next_col.size_of_data(), 0);
    assert_eq!(*next_col.col_name(), ok_name);
}

#[test]
fn test_save_to_file_wrong_paths()
{
    let empty_path = String::from("");
    let incorrect_path = String::from("./not_folder");

    let col_name = String::from("col_name");
    let col_h = ColHeader::new_empty(AllowedColType::IntType, col_name).unwrap();

    assert!(col_h.save_to_file(&empty_path).is_err());
    assert!(col_h.save_to_file(&incorrect_path).is_err());
}
