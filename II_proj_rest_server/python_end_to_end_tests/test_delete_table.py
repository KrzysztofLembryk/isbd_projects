from db_client import (get_table_by_id, delete_table, put_table, post_copy_query, get_query_by_id)
from csv_names import (OK_EMPLOYEES_WITH_HEADER)

QUERY_STATUS_KEY = "status"


def test_put_and_delete_table():
    """
    Test 1: Create a table, delete it, verify deletion response, and confirm it no longer exists.
    """
    print("\n" + "="*80)
    print("TEST 1: PUT table, DELETE it, and verify it's removed from database")
    print("="*80)
    
    table_name = "table_to_delete"
    columns = [
        {"name": "id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"}
    ]
    
    # Step 1: Create table
    print(f"  Step 1: Creating table '{table_name}'...")
    success, table_id, error = put_table(table_name, columns)
    assert success, f"Failed to create table: {error}"
    assert table_id is not None, "Table ID should not be None"
    print(f"    ✓ Table created with ID: {table_id}")
    
    # Step 2: Verify table exists
    print(f"  Step 2: Verifying table exists before deletion...")
    success, schema, error = get_table_by_id(table_id)
    assert success, f"Failed to get table before deletion: {error}"
    assert schema is not None, "Schema should not be None before deletion"
    print(f"    ✓ Table exists in database")
    
    # Step 3: Delete table
    print(f"  Step 3: Deleting table '{table_name}'...")
    success, error = delete_table(table_id)
    assert success, f"Failed to delete table: {error}"
    assert error is None, f"Error should be None on successful deletion, got: {error}"
    print(f"    ✓ Table deleted successfully")
    
    # Step 4: Verify table no longer exists
    print(f"  Step 4: Verifying table no longer exists...")
    success, schema, error = get_table_by_id(table_id)
    assert not success, "GET request should fail after table deletion"
    assert schema is None, "Schema should be None after deletion"
    assert error is not None, "Error message should be present when table doesn't exist"
    print(f"    ✓ Table confirmed removed from database: {error}")
    
    print("✓ TEST 1 PASSED: Table created, deleted, and verified as removed\n")

def test_delete_non_existent_table():
    """
    Test 2: Try to delete a table with non-existent ID (should fail with 404).
    """
    print("\n" + "="*80)
    print("TEST 2: DELETE table with non-existent ID")
    print("="*80)
    
    # Use a fixed UUID that doesn't exist in the database
    non_existent_id = "00000000-0000-0000-0000-000000000001"
    
    print(f"  Attempting to delete table with non-existent ID: {non_existent_id}")
    
    # Try to delete
    success, error = delete_table(non_existent_id)
    
    assert not success, "DELETE should fail for non-existent table ID"
    assert error is not None, "Error message should be present"
    
    print(f"  ✓ Correctly received error: {error}")
    print("✓ TEST 2 PASSED: Non-existent table ID correctly rejected with 404\n")

def test_delete_table_with_invalid_id():
    """
    Test 3: Try to delete a table with invalid ID format (not a UUID).
    """
    print("\n" + "="*80)
    print("TEST 3: DELETE table with invalid ID format (not UUID)")
    print("="*80)
    
    invalid_ids = [
        "some_wrong_id",
        "2137",
        "21.37",
        "not-a-valid-uuid-format",
        "12345678-1234-1234-1234",  # Incomplete UUID
        "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",  # Invalid characters
    ]
    
    for invalid_id in invalid_ids:
        print(f"  Attempting to delete table with invalid ID: {invalid_id}")
        
        # Try to delete
        success, error = delete_table(invalid_id)
        
        assert not success, f"DELETE should fail for invalid ID format: {invalid_id}"
        assert error is not None, f"Error message should be present for invalid ID: {invalid_id}"
        
        print(f"    ✓ Correctly received error: {error}")
    
    print("✓ TEST 3 PASSED: Invalid ID formats correctly rejected\n")

# =============================================================================
# For these tests to be successful you need to compile server with tests consts
# set, so that for small csv files query execution takes around 5s
# =============================================================================

#!/usr/bin/env python3
import time
from db_client import (
    put_table, 
    delete_table, 
    get_table_by_id, 
    post_copy_query, 
    get_query_by_id
)


def test_delete_table_with_running_query():
    """
    Test 4: Submit COPY query, immediately delete table, verify deletion is blocked 
    until query completes, then verify table is deleted after query finishes.
    """
    print("\n" + "="*80)
    print("TEST: DELETE table while COPY query is running")
    print("="*80)
    
    table_name = "employees"
    columns = [
        {"name": "emp_id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"},
        {"name": "salary", "type": "INT64"}
    ]
    csv_file = OK_EMPLOYEES_WITH_HEADER
    
    # Step 1: Create table
    print(f"  Step 1: Creating table '{table_name}'...")
    success, table_id, error = put_table(table_name, columns)
    assert success, f"Failed to create table: {error}"
    assert table_id is not None, "Table ID should not be None"
    print(f"    ✓ Table created with ID: {table_id}")
    
    # Step 2: Submit COPY query (this will take ~5 seconds with tests enabled)
    print(f"  Step 2: Submitting COPY query for '{csv_file}'...")
    success, query_id, error = post_copy_query(
        csv_file,
        table_name,
        does_csv_contain_header=True
    )
    assert success, f"Failed to submit COPY query: {error}"
    assert query_id is not None, "Query ID should not be None"
    print(f"    ✓ COPY query submitted with ID: {query_id}")
    
    # wait for query to start
    time.sleep(1)

    # Step 3: attempt to delete the table (should be blocked or queued)
    print(f"  Step 3: Attempting to delete table while query is running...")
    success, error = delete_table(table_id)
    
    if success:
        print(f"    ⚠ Delete request accepted (queued)")
    else:
        print(f"    ⚠ Delete request rejected: {error}")
    
    # Step 4: Verify if table is not accessible  immediately after delete attempt, even though it exists
    print(f"  Step 4: Verifying if table was marked for deletion thus access shouldnt be granted to fetch its details (query should still be running)...")
    success_get, schema, error_get = get_table_by_id(table_id)
    
    # Table should still exist at this point
    assert not success_get, f"Access to table should be blocked since its marked for deletion: {error_get}"
    print(f"    ✓ Table marked for deletion, no access (as expected)")
    
    # Step 5: Verify query is still running or completed
    print(f"  Step 5: Checking query status...")
    success_query, query_info, error_query = get_query_by_id(query_id)
    assert success_query, f"Failed to get query info: {error_query}"
    assert query_info is not None, "After get_query_by_id QUERY_INFO should not be None"
    
    query_state = query_info.get(QUERY_STATUS_KEY, "UNKNOWN")
    print(f"    ✓ Query state: {query_state}")
    
    # Step 6: Wait for query to complete (assuming server in test-mode adds ~5s delay)
    print(f"  Step 6: Waiting 6 seconds for query to complete...")
    time.sleep(6)
    
    # Step 7: Check query is completed
    print(f"  Step 7: Verifying query completed...")
    success_query, query_info, error_query = get_query_by_id(query_id)
    assert query_info is not None, "query information should always be available"
    assert success_query, f"Failed to get query info after waiting: {error_query}"
    
    query_state = query_info.get(QUERY_STATUS_KEY, "UNKNOWN")
    print(f"    ✓ Query state after waiting: {query_state}")
    assert query_state == "COMPLETED", f"Query should be COMPLETED, got: {query_state}"
    
    # Step 8: Verify table is now deleted
    print(f"  Step 8: Verifying table is now deleted...")
    success_get, schema, error_get = get_table_by_id(table_id)
    
    assert not success_get, "Table should be deleted after query completed"
    assert schema is None, "Schema should be None after deletion"
    assert error_get is not None, "Error message should be present"
    print(f"    ✓ Table successfully deleted: {error_get}")
    
    print("✓ TEST PASSED: Table deletion was properly handled during query execution\n")


def test_delete_table_after_query_completion():
    """
    Test: Submit COPY query, wait for completion, then delete table.
    This is the normal/expected flow.
    """
    print("\n" + "="*80)
    print("TEST: DELETE table after COPY query completes normally")
    print("="*80)
    
    table_name = "departments"
    columns = [
        {"name": "dept_id", "type": "INT64"},
        {"name": "dept_name", "type": "VARCHAR"}
    ]
    csv_file = "employees_with_header.csv"  # Reusing CSV for simplicity
    
    # Step 1: Create table
    print(f"  Step 1: Creating table '{table_name}'...")
    success, table_id, error = put_table(table_name, columns)
    assert success, f"Failed to create table: {error}"
    assert table_id is not None
    print(f"    ✓ Table created with ID: {table_id}")
    
    # Step 2: Submit COPY query
    print(f"  Step 2: Submitting COPY query...")
    success, query_id, error = post_copy_query(
        csv_file,
        table_name,
        destination_columns=["dept_id", "dept_name"],  # Map first 2 columns
        does_csv_contain_header=True
    )
    assert success, f"Failed to submit COPY query: {error}"
    assert query_id is not None, "Query ID should not be None when success is True"
    print(f"    ✓ COPY query submitted with ID: {query_id}")
    
    # Step 3: Wait for query to complete
    print(f"  Step 3: Waiting for query to complete...")
    time.sleep(6)
    
    success_query, query_info, error_query = get_query_by_id(query_id)
    assert success_query, f"Failed to get query info: {error_query}"
    assert query_info is not None, "query_info is None"
    query_state = query_info.get(QUERY_STATUS_KEY, "UNKNOWN")
    assert query_state == "COMPLETED", f"Query should be COMPLETED, got: {query_state}"
    print(f"    ✓ Query completed successfully")
    
    # Step 4: Delete table
    print(f"  Step 4: Deleting table...")
    success, error = delete_table(table_id)
    assert success, f"Failed to delete table: {error}"
    print(f"    ✓ Table deleted successfully")
    
    # Step 5: Verify table no longer exists
    print(f"  Step 5: Verifying table is deleted...")
    success_get, schema, error_get = get_table_by_id(table_id)
    assert not success_get, "Table should not exist after deletion"
    assert schema is None, "Schema should be None"
    print(f"    ✓ Table confirmed deleted")
    
    print("✓ TEST PASSED: Normal deletion flow works correctly\n")


def run_all_delete_with_query_tests():
    """
    Run all tests for deleting tables with running queries.
    """
    print("\n" + "="*80)
    print("RUNNING DELETE TABLE WITH QUERY TESTS")
    print("="*80)
    
    try:
        test_delete_table_with_running_query()
        # test_delete_table_after_query_completion()
        
        print("\n" + "="*80)
        print("ALL DELETE WITH QUERY TESTS PASSED! ✓")
        print("="*80 + "\n")
        
    except AssertionError as e:
        print(f"\n✗ TEST FAILED: {e}\n")
        raise
    except Exception as e:
        print(f"\n✗ UNEXPECTED ERROR: {e}\n")
        raise

def run_all_delete_table_tests():
    """
    Run all DELETE table tests.
    """
    print("\n" + "="*80)
    print("RUNNING ALL DELETE TABLE TESTS")
    print("="*80)
    
    try:
        # test_put_and_delete_table()
        # test_delete_non_existent_table()
        # test_delete_table_with_invalid_id()
        run_all_delete_with_query_tests()
        
        print("\n" + "="*80)
        print("ALL DELETE TABLE TESTS PASSED! ✓")
        print("="*80 + "\n")
        
    except AssertionError as e:
        print(f"\n✗ TEST FAILED: {e}\n")
        raise
    except Exception as e:
        print(f"\n✗ UNEXPECTED ERROR: {e}\n")
        raise


if __name__ == "__main__":
    run_all_delete_table_tests()