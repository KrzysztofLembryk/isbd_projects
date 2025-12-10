import time
from csv_names import (
    OK_EMPLOYEES_WITH_HEADER,
    OK_EMPLOYEES_NO_HEADER,
    WRONG_EMPLOYEES_WRONG_COLUMN_NAME,
    WRONG_EMPLOYEES_NO_HEADER_LESS_COLUMNS,
    WRONG_EMPLOYEES_NO_HEADER_MORE_COLUMNS,
    WRONG_EMPLOYEES_WITH_HEADER_LESS_COLUMNS,
    WRONG_EMPLOYEES_WITH_HEADER_MORE_COLUMNS,
    WRONG_EMPLOYEES_STR_INSTEAD_OF_INT,
    WRONG_EMPLOYEES_TOO_MANY_VALUES_IN_ROW,
    WRONG_EMPLOYEES_TOO_FEW_VALUES_IN_ROW,
    WRONG_EMPLOYEES_ONLY_HEADER,
    WRONG_EMPLOYEES_EMPTY
)
from db_client import (
    put_table, 
    delete_table, 
    get_table_by_id, 
    post_copy_query,
    post_select_query,
    get_query_by_id,
    get_query_result
)

QUERY_STATUS_KEY = "status"


def test_copy_query_success():
    """
    Test 1: Successful COPY query with correct CSV and table schema.
    """
    print("\n" + "="*80)
    print("TEST 1: COPY query success - correct CSV with header")
    print("="*80)
    
    table_name = "employees"
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
        OK_EMPLOYEES_WITH_HEADER,
        table_name,
        does_csv_contain_header=True
    )
    assert success and query_id, f"Failed to submit COPY query: {error}"
    
    # Wait for completion
    print("  Waiting for query completion...")
    time.sleep(6)
    
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
    time.sleep(6)
    
    # Get result
    print("  Fetching query result...")
    success, result, error = get_query_result(select_query_id, row_limit=10)
    assert success and result, f"Failed to get result: {error}"
    
    # Verify data
    assert result["rowCount"] == 10, f"Expected 10 rows, got: {result['rowCount']}"
    assert len(result["columns"]) == 3, f"Expected 3 columns, got: {len(result['columns'])}"
    
    print(f"  ✓ Data verified: {result['rowCount']} rows, {len(result['columns'])} columns")
    print("✓ TEST 1 PASSED\n")
    
    delete_table(table_id)


def run_all_copy_query_tests():
    """
    Run all COPY query tests.
    """
    print("\n" + "="*80)
    print("RUNNING ALL COPY QUERY TESTS")
    print("="*80)
    
    try:
        test_copy_query_success()
        
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
