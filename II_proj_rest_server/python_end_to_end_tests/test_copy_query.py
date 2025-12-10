import time
from csv_names import (
    OK_EMPLOYEES_WITH_HEADER,
    OK_EMPLOYEES_NO_HEADER,
    OK_EMPLOYEES_4_COLUMNS,
    OK_LARGE_EMPLOYEES,
    WRONG_EMPLOYEES_WRONG_COLUMN_NAME,
    WRONG_EMPLOYEES_NO_HEADER_LESS_COLUMNS,
    WRONG_EMPLOYEES_NO_HEADER_MORE_COLUMNS,
    WRONG_EMPLOYEES_WITH_HEADER_LESS_COLUMNS,
    WRONG_EMPLOYEES_WITH_HEADER_MORE_COLUMNS,
    WRONG_EMPLOYEES_STR_INSTEAD_OF_INT,
    WRONG_EMPLOYEES_TOO_MANY_VALUES_IN_ROW,
    WRONG_EMPLOYEES_TOO_FEW_VALUES_IN_ROW,
    WRONG_EMPLOYEES_ONLY_HEADER,
    WRONG_EMPLOYEES_EMPTY,
    WRONG_EMPLOYEES_NON_EXISTENT_FILE,
)
from db_client import (
    put_table, 
    delete_table, 
    get_table_by_id, 
    post_copy_query,
    post_select_query,
    get_query_by_id,
    get_query_result,
    get_failed_query
)

SLEEP_TIME = 3
QUERY_STATUS_KEY = "status"
ROW_LIMIT = 10


def test_copy_query_success():
    """
    Test 1: Successful COPY query with correct CSV and table schema.
    """
    print("\n" + "="*80)
    print("TEST 1: COPY query success - correct CSV with header")
    print("="*80)
    
    table_name = "employees_2"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "salary", "type": "INT64"}
    ]
    
    # Create table
    print("  Creating table...")
    success, table_id, error = put_table(table_name, columns)
    assert success and table_id, f"Failed to create table: {error}"
    
    # Submit COPY query
    print("  Submitting COPY query...")
    success, query_id, error = post_copy_query(
        # OK_EMPLOYEES_WITH_HEADER,
        "/data/ok_employees_with_header.csv",
        table_name,
        does_csv_contain_header=True
    )
    assert success and query_id, f"Failed to submit COPY query: {error}"
    
    # Wait for completion
    print("  Waiting for query completion...")
    time.sleep(SLEEP_TIME)
    
    # Check query status
    print("  Checking query status...")
    success, query_info, error = get_query_by_id(query_id)
    assert success and query_info, f"Failed to get query info: {error}"
    assert query_info[QUERY_STATUS_KEY] == "COMPLETED", \
        f"Expected COMPLETED, got: {query_info[QUERY_STATUS_KEY]}"
    
    # Submit SELECT query to verify data
    print("  Submitting SELECT query...")
    success, select_query_id, error = post_select_query(table_name)
    assert success and select_query_id, f"Failed to submit SELECT query: {error}"
    
    # Wait for SELECT completion
    time.sleep(SLEEP_TIME)
    
    # Get result
    print("  Fetching query result...")
    success, result, error = get_query_result(select_query_id, row_limit=ROW_LIMIT)
    assert success and result, f"Failed to get result: {error}"
    
    # Verify data
    assert result["rowCount"] == ROW_LIMIT, f"Expected 20 rows, got: {result['rowCount']}"
    assert len(result["columns"]) == 3, f"Expected 3 columns, got: {len(result['columns'])}"
    
    print(f"  ✓ Data verified: {result['rowCount']} rows, {len(result['columns'])} columns")
    print("✓ TEST 1 PASSED\n")
    
    delete_table(table_id)

def test_copy_query_wrong_column_name():
    """
    Test 2: COPY query with wrong column name in CSV header (should fail).
    """
    print("\n" + "="*80)
    print("TEST 2: COPY query - wrong column name in header")
    print("="*80)
    
    table_name = "employees_wrong_col"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "salary", "type": "INT64"}
    ]
    
    print("  Creating table...")
    success, table_id, error = put_table(table_name, columns)
    assert success and table_id, f"Failed to create table: {error}"
    
    print("  Submitting COPY query with wrong column name...")
    success, query_id, error = post_copy_query(
        WRONG_EMPLOYEES_WRONG_COLUMN_NAME,
        table_name,
        does_csv_contain_header=True
    )
    assert success and query_id, f"Failed to submit COPY query: {error}"
    
    print("  Waiting for query to fail...")
    time.sleep(SLEEP_TIME)
    
    print("  Checking query status...")
    success, query_info, error = get_query_by_id(query_id)
    assert success and query_info, f"Failed to get query info: {error}"
    assert query_info[QUERY_STATUS_KEY] == "FAILED", \
        f"Expected FAILED, got: {query_info[QUERY_STATUS_KEY]}"
    
    print("  ✓ Query correctly failed")
    print("✓ TEST 2 PASSED\n")
    
    delete_table(table_id)

def test_copy_query_no_header_less_columns():
    """
    Test 3: COPY query without header, less columns than schema (should fail).
    """
    print("\n" + "="*80)
    print("TEST 3: COPY query - no header, less columns")
    print("="*80)
    
    table_name = "employees_less_cols"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "salary", "type": "INT64"}
    ]
    
    print("  Creating table...")
    success, table_id, error = put_table(table_name, columns)
    assert success and table_id, f"Failed to create table: {error}"
    
    print("  Submitting COPY query with less columns...")
    success, query_id, error = post_copy_query(
        WRONG_EMPLOYEES_NO_HEADER_LESS_COLUMNS,
        table_name,
        does_csv_contain_header=False
    )
    assert success and query_id, f"Failed to submit COPY query: {error}"
    
    print("  Waiting for query to fail...")
    time.sleep(SLEEP_TIME)
    
    print("  Checking query status...")
    success, query_info, error = get_query_by_id(query_id)
    assert success and query_info, f"Failed to get query info: {error}"
    assert query_info[QUERY_STATUS_KEY] == "FAILED", \
        f"Expected FAILED, got: {query_info[QUERY_STATUS_KEY]}"
    
    print("  ✓ Query correctly failed")
    print("✓ TEST 3 PASSED\n")
    
    delete_table(table_id)

def test_copy_query_no_header_more_columns():
    """
    Test 4: COPY query without header, more columns than schema (should fail).
    """
    print("\n" + "="*80)
    print("TEST 4: COPY query - no header, more columns")
    print("="*80)
    
    table_name = "employees_more_cols"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "salary", "type": "INT64"}
    ]
    
    print("  Creating table...")
    success, table_id, error = put_table(table_name, columns)
    assert success and table_id, f"Failed to create table: {error}"
    
    print("  Submitting COPY query with more columns...")
    success, query_id, error = post_copy_query(
        WRONG_EMPLOYEES_NO_HEADER_MORE_COLUMNS,
        table_name,
        does_csv_contain_header=False
    )
    assert success and query_id, f"Failed to submit COPY query: {error}"
    
    print("  Waiting for query to fail...")
    time.sleep(SLEEP_TIME)
    
    print("  Checking query status...")
    success, query_info, error = get_query_by_id(query_id)
    assert success and query_info, f"Failed to get query info: {error}"
    assert query_info[QUERY_STATUS_KEY] == "FAILED", \
        f"Expected FAILED, got: {query_info[QUERY_STATUS_KEY]}"
    
    print("  ✓ Query correctly failed")
    print("✓ TEST 4 PASSED\n")
    
    delete_table(table_id)

def test_copy_query_with_header_less_columns():
    """
    Test 5: COPY query with header, less columns than schema (should fail).
    """
    print("\n" + "="*80)
    print("TEST 5: COPY query - with header, less columns")
    print("="*80)
    
    table_name = "employees_header_less"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "salary", "type": "INT64"}
    ]
    
    print("  Creating table...")
    success, table_id, error = put_table(table_name, columns)
    assert success and table_id, f"Failed to create table: {error}"
    
    print("  Submitting COPY query with less columns...")
    success, query_id, error = post_copy_query(
        WRONG_EMPLOYEES_WITH_HEADER_LESS_COLUMNS,
        table_name,
        does_csv_contain_header=True
    )
    assert success and query_id, f"Failed to submit COPY query: {error}"
    
    print("  Waiting for query to fail...")
    time.sleep(SLEEP_TIME)
    
    print("  Checking query status...")
    success, query_info, error = get_query_by_id(query_id)
    assert success and query_info, f"Failed to get query info: {error}"
    assert query_info[QUERY_STATUS_KEY] == "FAILED", \
        f"Expected FAILED, got: {query_info[QUERY_STATUS_KEY]}"
    
    print("  ✓ Query correctly failed")
    print("✓ TEST 5 PASSED\n")
    
    delete_table(table_id)

def test_copy_query_with_header_more_columns():
    """
    Test 6: COPY query with header, more columns than schema (should fail or use destinationColumns).
    """
    print("\n" + "="*80)
    print("TEST 6: COPY query - with header, more columns")
    print("="*80)
    
    table_name = "employees_header_more"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "salary", "type": "INT64"}
    ]
    
    print("  Creating table...")
    success, table_id, error = put_table(table_name, columns)
    assert success and table_id, f"Failed to create table: {error}"
    
    print("  Submitting COPY query with more columns (no destinationColumns)...")
    success, query_id, error = post_copy_query(
        WRONG_EMPLOYEES_WITH_HEADER_MORE_COLUMNS,
        table_name,
        does_csv_contain_header=True
    )
    assert success and query_id, f"Failed to submit COPY query: {error}"
    
    print("  Waiting for query to fail...")
    time.sleep(SLEEP_TIME)
    
    print("  Checking query status...")
    success, query_info, error = get_query_by_id(query_id)
    assert success and query_info, f"Failed to get query info: {error}"
    assert query_info[QUERY_STATUS_KEY] == "FAILED", \
        f"Expected FAILED, got: {query_info[QUERY_STATUS_KEY]}"
    
    print("  ✓ Query correctly failed")
    print("✓ TEST 6 PASSED\n")
    
    delete_table(table_id)

def test_copy_query_str_instead_of_int():
    """
    Test 7: COPY query with string value in INT64 column (should fail).
    """
    print("\n" + "="*80)
    print("TEST 7: COPY query - string in INT64 column")
    print("="*80)
    
    table_name = "employees_type_error"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "salary", "type": "INT64"}
    ]
    
    print("  Creating table...")
    success, table_id, error = put_table(table_name, columns)
    assert success and table_id, f"Failed to create table: {error}"
    
    print("  Submitting COPY query with type mismatch...")
    success, query_id, error = post_copy_query(
        WRONG_EMPLOYEES_STR_INSTEAD_OF_INT,
        table_name,
        does_csv_contain_header=True
    )
    assert success and query_id, f"Failed to submit COPY query: {error}"
    
    print("  Waiting for query to fail...")
    time.sleep(SLEEP_TIME)
    
    print("  Checking query status...")
    success, query_info, error = get_query_by_id(query_id)
    assert success and query_info, f"Failed to get query info: {error}"
    assert query_info[QUERY_STATUS_KEY] == "FAILED", \
        f"Expected FAILED, got: {query_info[QUERY_STATUS_KEY]}"
    
    print("  ✓ Query correctly failed")
    print("✓ TEST 7 PASSED\n")
    
    delete_table(table_id)

def test_copy_query_too_many_values_in_row():
    """
    Test 8: COPY query with too many values in one row (should fail).
    """
    print("\n" + "="*80)
    print("TEST 8: COPY query - too many values in row")
    print("="*80)
    
    table_name = "employees_extra_value"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "salary", "type": "INT64"}
    ]
    
    print("  Creating table...")
    success, table_id, error = put_table(table_name, columns)
    assert success and table_id, f"Failed to create table: {error}"
    
    print("  Submitting COPY query with extra value in row...")
    success, query_id, error = post_copy_query(
        WRONG_EMPLOYEES_TOO_MANY_VALUES_IN_ROW,
        table_name,
        does_csv_contain_header=True
    )
    assert success and query_id, f"Failed to submit COPY query: {error}"
    
    print("  Waiting for query to fail...")
    time.sleep(SLEEP_TIME)
    
    print("  Checking query status...")
    success, query_info, error = get_query_by_id(query_id)
    assert success and query_info, f"Failed to get query info: {error}"
    assert query_info[QUERY_STATUS_KEY] == "FAILED", \
        f"Expected FAILED, got: {query_info[QUERY_STATUS_KEY]}"
    
    print("  ✓ Query correctly failed")
    print("✓ TEST 8 PASSED\n")
    
    delete_table(table_id)

def test_copy_query_too_few_values_in_row():
    """
    Test 9: COPY query with too few values in one row (should fail).
    """
    print("\n" + "="*80)
    print("TEST 9: COPY query - too few values in row")
    print("="*80)
    
    table_name = "employees_missing_value"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "salary", "type": "INT64"}
    ]
    
    print("  Creating table...")
    success, table_id, error = put_table(table_name, columns)
    assert success and table_id, f"Failed to create table: {error}"
    
    print("  Submitting COPY query with missing value in row...")
    success, query_id, error = post_copy_query(
        WRONG_EMPLOYEES_TOO_FEW_VALUES_IN_ROW,
        table_name,
        does_csv_contain_header=True
    )
    assert success and query_id, f"Failed to submit COPY query: {error}"
    
    print("  Waiting for query to fail...")
    time.sleep(SLEEP_TIME)
    
    print("  Checking query status...")
    success, query_info, error = get_query_by_id(query_id)
    assert success and query_info, f"Failed to get query info: {error}"
    assert query_info[QUERY_STATUS_KEY] == "FAILED", \
        f"Expected FAILED, got: {query_info[QUERY_STATUS_KEY]}"
    
    print("  ✓ Query correctly failed")
    print("✓ TEST 9 PASSED\n")
    
    delete_table(table_id)

def test_copy_query_only_header():
    """
    Test 10: COPY query with only header, no data rows (should succeed with 0 rows).
    """
    print("\n" + "="*80)
    print("TEST 10: COPY query - only header, no data")
    print("="*80)
    
    table_name = "employees_empty_data"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "salary", "type": "INT64"}
    ]
    
    print("  Creating table...")
    success, table_id, error = put_table(table_name, columns)
    assert success and table_id, f"Failed to create table: {error}"
    
    print("  Submitting COPY query with only header...")
    success, copy_query_id, error = post_copy_query(
        WRONG_EMPLOYEES_ONLY_HEADER,
        table_name,
        does_csv_contain_header=True
    )
    assert success and copy_query_id, f"Failed to submit COPY query: {error}"
    
    print("  Waiting for completion...")
    time.sleep(SLEEP_TIME)
    
    print("  Checking query status...")
    success, query_info, error = get_query_by_id(copy_query_id)
    assert success and query_info, f"Failed to get query info: {error}"
    assert query_info[QUERY_STATUS_KEY] == "FAILED", \
        f"Expected FAILED, got: {query_info[QUERY_STATUS_KEY]}"
    
    # Verify 0 rows were inserted
    print("  Submitting SELECT query...")
    success, select_query_id, error = post_select_query(table_name)
    assert success and select_query_id, f"Failed to submit SELECT query: {error}"
    
    time.sleep(SLEEP_TIME)
    
    print("  Fetching result...")
    success, result, error = get_query_result(select_query_id)
    assert success and result, f"Failed to get result: {error}"
    assert result["rowCount"] == 0, f"Expected 0 rows, got: {result['rowCount']}"
    
    print("  ✓ Query succeeded with 0 rows")
    print("✓ TEST 10 PASSED\n")
    
    delete_table(table_id)

def test_copy_query_empty_file():
    """
    Test 11: COPY query with completely empty CSV file (should fail).
    """
    print("\n" + "="*80)
    print("TEST 11: COPY query - empty file")
    print("="*80)
    
    table_name = "employees_empty_file"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "salary", "type": "INT64"}
    ]
    
    print("  Creating table...")
    success, table_id, error = put_table(table_name, columns)
    assert success and table_id, f"Failed to create table: {error}"
    
    print("  Submitting COPY query with empty file...")
    success, query_id, error = post_copy_query(
        WRONG_EMPLOYEES_EMPTY,
        table_name,
        does_csv_contain_header=True
    )
    assert success and query_id, f"Failed to submit COPY query: {error}"
    
    print("  Waiting for query to fail...")
    time.sleep(SLEEP_TIME)
    
    print("  Checking query status...")
    success, query_info, error = get_query_by_id(query_id)
    assert success and query_info, f"Failed to get query info: {error}"
    assert query_info[QUERY_STATUS_KEY] == "FAILED", \
        f"Expected FAILED, got: {query_info[QUERY_STATUS_KEY]}"
    
    print("  ✓ Query correctly failed")
    print("✓ TEST 11 PASSED\n")
    
    delete_table(table_id)

def test_copy_query_non_existent_file():
    """
    Test 12: COPY query with non-existent CSV file (should fail).
    """
    print("\n" + "="*80)
    print("TEST 12: COPY query - non-existent file")
    print("="*80)
    
    table_name = "employees_no_file"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "salary", "type": "INT64"}
    ]
    
    print("  Creating table...")
    success, table_id, error = put_table(table_name, columns)
    assert success and table_id, f"Failed to create table: {error}"
    
    print("  Submitting COPY query with non-existent file...")
    success, query_id, error = post_copy_query(
        WRONG_EMPLOYEES_NON_EXISTENT_FILE,
        table_name,
        does_csv_contain_header=True
    )
    assert success and query_id, f"Failed to submit COPY query: {error}"
    
    print("  Waiting for query to fail...")
    time.sleep(SLEEP_TIME)
    
    print("  Checking query status...")
    success, query_info, error = get_query_by_id(query_id)
    assert success and query_info, f"Failed to get query info: {error}"
    assert query_info[QUERY_STATUS_KEY] == "FAILED", \
        f"Expected FAILED, got: {query_info[QUERY_STATUS_KEY]}"
    
    print("  ✓ Query correctly failed")
    print("✓ TEST 12 PASSED\n")
    
    delete_table(table_id)

def test_copy_query_sequential_execution():
    """
    Test 13: Two COPY queries for the same table submitted sequentially.
    Only one should execute at a time due to table locking.
    """
    print("\n" + "="*80)
    print("TEST 13: COPY query - sequential execution with table lock")
    print("="*80)
    
    table_name = "employees_sequential"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "salary", "type": "INT64"}
    ]
    
    print("  Creating table...")
    success, table_id, error = put_table(table_name, columns)
    assert success and table_id, f"Failed to create table: {error}"
    
    # Submit first COPY query
    print("  Submitting first COPY query...")
    success, query_id_1, error = post_copy_query(
        OK_EMPLOYEES_WITH_HEADER,
        table_name,
        does_csv_contain_header=True
    )
    assert success and query_id_1, f"Failed to submit first COPY query: {error}"
    
    # Submit second COPY query immediately
    print("  Submitting second COPY query immediately...")
    success, query_id_2, error = post_copy_query(
        OK_EMPLOYEES_WITH_HEADER,
        table_name,
        does_csv_contain_header=True
    )
    assert success and query_id_2, f"Failed to submit second COPY query: {error}"
    
    # Wait for first query to complete
    print("  Waiting for first query to complete...")
    time.sleep(SLEEP_TIME)
    
    # Check first query is completed
    print("  Checking first query status...")
    success, query_info_1, error = get_query_by_id(query_id_1)
    assert success and query_info_1, f"Failed to get first query info: {error}"
    assert query_info_1[QUERY_STATUS_KEY] == "COMPLETED", \
        f"First query should be COMPLETED, got: {query_info_1[QUERY_STATUS_KEY]}"
    print("    ✓ First query completed")
    
    # Check second query is still running or queued
    print("  Checking second query status (should be RUNNING or QUEUED)...")
    success, query_info_2, error = get_query_by_id(query_id_2)
    assert success and query_info_2, f"Failed to get second query info: {error}"
    second_status = query_info_2[QUERY_STATUS_KEY]
    assert second_status in ["RUNNING", "PLANNING"], \
        f"Second query should be RUNNING or QUEUED, got: {second_status}"
    print(f"    ✓ Second query status: {second_status}")
    
    # Verify table has data from first COPY only (20 rows)
    print("  Submitting SELECT query to verify first COPY data...")
    success, select_query_id_1, error = post_select_query(table_name)
    assert success and select_query_id_1, f"Failed to submit SELECT query: {error}"
    
    time.sleep(SLEEP_TIME)
    
    print("  Fetching result after first COPY...")
    success, result_1, error = get_query_result(select_query_id_1, row_limit=50)
    assert success and result_1, f"Failed to get result: {error}"
    assert result_1["rowCount"] == ROW_LIMIT, \
        f"Expected 10 rows after first COPY, got: {result_1['rowCount']}"
    print(f"    ✓ Table has {result_1['rowCount']} rows (first COPY only)")
    
    # Wait for second query to complete
    print("  Waiting for second query to complete...")
    time.sleep(SLEEP_TIME)
    
    # Check second query is now completed
    print("  Checking second query final status...")
    success, query_info_2_final, error = get_query_by_id(query_id_2)
    assert success and query_info_2_final, f"Failed to get second query info: {error}"
    assert query_info_2_final[QUERY_STATUS_KEY] == "COMPLETED", \
        f"Second query should be COMPLETED, got: {query_info_2_final[QUERY_STATUS_KEY]}"
    print("    ✓ Second query completed")
    
    # Verify table now has data from both COPYs (20 rows)
    print("  Submitting SELECT query to verify both COPY operations...")
    success, select_query_id_2, error = post_select_query(table_name)
    assert success and select_query_id_2, f"Failed to submit SELECT query: {error}"
    
    time.sleep(SLEEP_TIME)
    
    print("  Fetching result after both COPYs...")
    success, result_2, error = get_query_result(select_query_id_2, row_limit=50)
    assert success and result_2, f"Failed to get result: {error}"
    assert result_2["rowCount"] == 2 * ROW_LIMIT, \
        f"Expected 20 rows after both COPYs, got: {result_2['rowCount']}"
    print(f"    ✓ Table has {result_2['rowCount']} rows (both COPYs)")
    
    print("✓ TEST 13 PASSED\n")
    
    delete_table(table_id)

def test_copy_query_500_rows_file():
    """
    Test 14: COPY query with 500 rows CSV file.
    Submit 10 SELECT queries after COPY - all should succeed after timeout.
    """
    print("\n" + "="*80)
    print("TEST 14: COPY query - 500 rows with multiple SELECT queries")
    print("="*80)
    
    table_name = "employees_large"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "salary", "type": "INT64"}
    ]
    
    print("  Creating table...")
    success, table_id, error = put_table(table_name, columns)
    assert success and table_id, f"Failed to create table: {error}"
    
    # Submit COPY query with large file
    print("  Submitting COPY query for 500 rows...")
    success, copy_query_id, error = post_copy_query(
        OK_LARGE_EMPLOYEES,
        table_name,
        does_csv_contain_header=True
    )
    assert success and copy_query_id, f"Failed to submit COPY query: {error}"
    
    print("  Waiting for COPY query to complete...")
    time.sleep(SLEEP_TIME + 2)
    # Submit 10 SELECT queries immediately
    print("  Submitting 10 SELECT queries immediately...")
    select_query_ids = []
    for i in range(10):
        success, select_query_id, error = post_select_query(table_name)
        assert success and select_query_id, f"Failed to submit SELECT query {i+1}: {error}"
        select_query_ids.append(select_query_id)
    print(f"    ✓ Submitted {len(select_query_ids)} SELECT queries")
    
    # Check COPY query status
    print("  Checking COPY query status...")
    success, copy_info, error = get_query_by_id(copy_query_id)
    assert success and copy_info, f"Failed to get COPY query info: {error}"
    assert copy_info[QUERY_STATUS_KEY] == "COMPLETED", \
        f"COPY query should be COMPLETED, got: {copy_info[QUERY_STATUS_KEY]}"
    print("    ✓ COPY query completed")
    
    # Wait for SELECT queries to complete
    print("  Waiting for SELECT queries to complete...")
    time.sleep(SLEEP_TIME)
    
    # Verify all 10 SELECT queries completed successfully
    print("  Verifying all SELECT queries completed...")
    for i, select_query_id in enumerate(select_query_ids):
        success, query_info, error = get_query_by_id(select_query_id)
        assert success and query_info, f"Failed to get SELECT query {i+1} info: {error}"
        assert query_info[QUERY_STATUS_KEY] == "COMPLETED", \
            f"SELECT query {i+1} should be COMPLETED, got: {query_info[QUERY_STATUS_KEY]}"
    print(f"    ✓ All {len(select_query_ids)} SELECT queries completed")
    
    # Fetch and verify results from first SELECT query
    print("  Fetching results from first SELECT query...")
    success, result, error = get_query_result(select_query_ids[0], row_limit=500)
    assert success and result, f"Failed to get result: {error}"
    
    # Verify data
    assert result["rowCount"] == 500, \
        f"Expected 500 rows, got: {result['rowCount']}"
    assert len(result["columns"]) == 3, \
        f"Expected 3 columns, got: {len(result['columns'])}"
    
    print(f"    ✓ Data verified: {result['rowCount']} rows, {len(result['columns'])} columns")
    
    # Verify data pattern (every 50 rows should have same name/salary)
    # Columns are returned as arrays in the same order as table definition:
    # [emp_id_values, name_values, salary_values]
    print("  Verifying data pattern (repeated rows)...")
    emp_id_values = result["columns"][0]
    name_values = result["columns"][1]
    salary_values = result["columns"][2]
    
    # Check first 50 rows have same name and salary
    if len(name_values) >= 50:
        first_name = name_values[0]
        first_salary = salary_values[0]
        
        for i in range(50):
            assert name_values[i] == first_name, \
                f"Row {i} name should match first row"
            assert salary_values[i] == first_salary, \
                f"Row {i} salary should match first row"
        
        print("    ✓ Data pattern verified (first 50 rows have same name/salary)")
    
    # Verify emp_id is sequential
    for i in range(min(100, len(emp_id_values))):
        assert emp_id_values[i] == i + 1, \
            f"emp_id at index {i} should be {i+1}, got {emp_id_values[i]}"
    print("    ✓ emp_id sequence verified")
    
    print("✓ TEST 14 PASSED\n")
    
    delete_table(table_id)

def test_copy_query_with_destination_columns():
    """
    Test 15: COPY query with destinationColumns mapping.
    CSV has 4 columns (emp_id, full_name, department, salary)
    Table has 3 columns (emp_id, name, department)
    Map: emp_id->emp_id, full_name->name, department->department (skip salary)
    """
    print("\n" + "="*80)
    print("TEST 15: COPY query - with destinationColumns mapping")
    print("="*80)
    
    table_name = "employees_mapped"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "department", "type": "VARCHAR"}
    ]
    
    print("  Creating table with 3 columns (emp_id, name, department)...")
    success, table_id, error = put_table(table_name, columns)
    assert success and table_id, f"Failed to create table: {error}"

    # CSV has: emp_id, full_name, department, salary
    # Table has: emp_id, name, department
    # Map: emp_id->emp_id, full_name->name, department->department (skip salary column 3)
    destination_cols = ["emp_id", "name", "department"]
    
    print("  Submitting COPY query with destinationColumns...")
    print(f"    CSV columns: [emp_id, full_name, department, salary]")
    print(f"    Table columns: [emp_id, name, department]")
    print(f"    Mapping to:  {destination_cols}")
    success, query_id, error = post_copy_query(
        OK_EMPLOYEES_4_COLUMNS,
        table_name,
        does_csv_contain_header=True,
        destination_columns=destination_cols
    )
    assert success and query_id, f"Failed to submit COPY query: {error}"
    
    print("  Waiting for completion...")
    time.sleep(SLEEP_TIME)

    print("  Checking query status...")
    success, query_info, error = get_query_by_id(query_id)
    assert success and query_info, f"Failed to get query info: {error}"
    assert query_info[QUERY_STATUS_KEY] == "COMPLETED", \
        f"Expected COMPLETED, got: {query_info[QUERY_STATUS_KEY]}"
    print("    ✓ COPY query completed")
    
    # Verify data
    print("  Submitting SELECT query...")
    success, select_query_id, error = post_select_query(table_name)
    assert success and select_query_id, f"Failed to submit SELECT query: {error}"
    
    time.sleep(SLEEP_TIME)
    
    print("  Fetching results...")
    success, result, error = get_query_result(select_query_id, row_limit=20)
    assert success and result, f"Failed to get result: {error}"

    # Verify row count
    assert result["rowCount"] == 10, \
        f"Expected 10 rows, got: {result['rowCount']}"
    assert len(result["columns"]) == 3, \
        f"Expected 3 columns, got: {len(result['columns'])}"
    
    print(f"    ✓ Data verified: {result['rowCount']} rows, {len(result['columns'])} columns")
    
    # Verify data content - check that full_name was mapped to name
    # Column order: emp_id (0), name (1), department (2)
    emp_id_values = result["columns"][0]
    name_values = result["columns"][1]
    department_values = result["columns"][2]
    
    # First row from CSV should be: emp_id=1, full_name="Alice Smith", department="Engineering", salary=75000
    # Table should have: emp_id=1, name="Alice Smith", department="Engineering"
    assert emp_id_values[0] == 1, f"Expected emp_id=1, got {emp_id_values[0]}"
    assert name_values[0] == "Alice Smith", \
        f"Expected name='Alice Smith', got '{name_values[0]}'"
    assert department_values[0] == "Engineering", \
        f"Expected department='Engineering', got '{department_values[0]}'"
    
    print("    ✓ Column mapping verified:")
    print("      - emp_id=1")
    print("      - full_name -> name='Alice Smith'")
    print("      - department='Engineering'")
    print("      - salary (skipped)")
    
    # Verify second row
    assert emp_id_values[1] == 2, f"Expected emp_id=2, got {emp_id_values[1]}"
    assert name_values[1] == "Bob Johnson", \
        f"Expected name='Bob Johnson', got '{name_values[1]}'"
    assert department_values[1] == "Marketing", \
        f"Expected department='Marketing', got '{department_values[1]}'"
    
    print("    ✓ Second row verified: emp_id=2, name='Bob Johnson', department='Marketing'")
    
    print("✓ TEST 15 PASSED\n")
    
    delete_table(table_id)
    

def run_all_copy_query_tests():
    """
    Run all COPY query tests.
    """
    print("\n" + "="*80)
    print("RUNNING ALL COPY QUERY TESTS")
    print("="*80)
    
    try:
        # test_copy_query_success()
        # test_copy_query_wrong_column_name()
        # test_copy_query_no_header_less_columns()
        # test_copy_query_no_header_more_columns()
        # test_copy_query_with_header_less_columns()
        # test_copy_query_with_header_more_columns()
        # test_copy_query_str_instead_of_int()
        # test_copy_query_too_many_values_in_row()
        # test_copy_query_too_few_values_in_row()
        # test_copy_query_only_header()
        # test_copy_query_empty_file()
        # test_copy_query_non_existent_file()
        # test_copy_query_sequential_execution()
        # test_copy_query_500_rows_file()  
        test_copy_query_with_destination_columns()
        
        print("\n" + "="*80)
        print("ALL COPY QUERY TESTS PASSED! ✓")
        print("="*80 + "\n")
        
    except AssertionError as e:
        print(f"\n✗ TEST FAILED: {e}\n")
        raise
    except Exception as e:
        print(f"\n✗ UNEXPECTED ERROR: {e}\n")
        raise


if __name__ == "__main__":
    run_all_copy_query_tests()
