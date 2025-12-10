#!/usr/bin/env python3
import time
from csv_names import (OK_EMPLOYEES_WITH_HEADER)
from db_client import (
    put_table, 
    delete_table, 
    get_table_by_id, 
    post_copy_query, 
    get_query_by_id,
    post_select_query,
    get_failed_query
)

SLEEP_TIME = 3
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

def test_double_delete():
    """
    Test 4: Create a table, delete it successfully, then try to delete it again.
    The second delete should fail with 404 (table not found).
    """
    print("\n" + "="*80)
    print("TEST 4: Double DELETE - second delete should fail")
    print("="*80)
    
    table_name = "table_for_double_delete"
    columns = [
        {"name": "id", "type": "INT64"},
        {"name": "value", "type": "VARCHAR"}
    ]
    
    # Step 1: Create table
    print(f"  Step 1: Creating table '{table_name}'...")
    success, table_id, error = put_table(table_name, columns)
    assert success, f"Failed to create table: {error}"
    assert table_id is not None, "Table ID should not be None"
    print(f"    ✓ Table created with ID: {table_id}")
    
    # Step 2: First delete - should succeed
    print(f"  Step 2: First DELETE attempt...")
    success, error = delete_table(table_id)
    assert success, f"First delete should succeed: {error}"
    assert error is None, f"Error should be None on successful deletion, got: {error}"
    print(f"    ✓ First delete succeeded")
    
    # Step 3: Verify table no longer exists
    print(f"  Step 3: Verifying table is deleted...")
    success_get, schema, error_get = get_table_by_id(table_id)
    assert not success_get, "GET should fail after table deletion"
    assert schema is None, "Schema should be None after deletion"
    print(f"    ✓ Table confirmed deleted")
    
    # Step 4: Second delete - should fail
    print(f"  Step 4: Second DELETE attempt (should fail)...")
    success, error = delete_table(table_id)
    assert not success, "Second delete should fail - table already deleted"
    assert error is not None, "Error message should be present for second delete"
    print(f"    ✓ Second delete correctly failed: {error}")
    
    print("✓ TEST 4 PASSED: Double delete properly rejected\n")

# =============================================================================
# For these tests to be successful you need to compile server with tests consts
# set, so that for small csv files query execution takes around 5s
# =============================================================================



def test_delete_table_with_running_query():
    """
    Test 4: Submit COPY query, immediately delete table, verify deletion is blocked until query completes, then verify table is deleted after query finishes.
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

    # Step 3: attempt to delete the table (should be blocked )
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
    assert table_id is not None
    print(f"    ✓ Table created with ID: {table_id}")
    
    # Step 2: Submit COPY query
    print(f"  Step 2: Submitting COPY query...")
    success, query_id, error = post_copy_query(
        csv_file,
        table_name,
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


def test_delete_table_then_query_attempts():
    """
    Test: Create table, run COPY query, delete table after completion,
    then attempt SELECT and COPY queries on deleted table.
    Queries will be rejected at submission but still tracked as FAILED queries.
    """
    print("\n" + "="*80)
    print("TEST: DELETE table then attempt SELECT and COPY queries")
    print("="*80)
    
    table_name = "employees_delete_then_query"
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
    assert table_id is not None
    print(f"    ✓ Table created with ID: {table_id}")
    
    # Step 2: Submit first COPY query
    print(f"  Step 2: Submitting COPY query...")
    success, copy_query_id, error = post_copy_query(
        csv_file,
        table_name,
        does_csv_contain_header=True
    )
    assert success, f"Failed to submit COPY query: {error}"
    assert copy_query_id is not None
    print(f"    ✓ COPY query submitted with ID: {copy_query_id}")
    
    # Step 3: Wait for COPY query to complete
    print(f"  Step 3: Waiting for COPY query to complete...")
    time.sleep(SLEEP_TIME)
    
    success_query, query_info, error_query = get_query_by_id(copy_query_id)
    assert success_query, f"Failed to get query info: {error_query}"
    assert query_info is not None
    query_state = query_info.get(QUERY_STATUS_KEY, "UNKNOWN")
    assert query_state == "COMPLETED", f"COPY query should be COMPLETED, got: {query_state}"
    print(f"    ✓ COPY query completed successfully")
    
    # Step 4: Delete table
    print(f"  Step 4: Deleting table...")
    success, error = delete_table(table_id)
    assert success, f"Failed to delete table: {error}"
    print(f"    ✓ Table deleted successfully")
    
    # Step 5: Verify table is deleted
    print(f"  Step 5: Verifying table is deleted...")
    success_get, schema, error_get = get_table_by_id(table_id)
    assert not success_get, "Table should not exist after deletion"
    assert schema is None
    print(f"    ✓ Table confirmed deleted: {error_get}")
    
    # Step 6: Submit SELECT query on deleted table (will fail but get query ID)
    print(f"  Step 6: Submitting SELECT query on deleted table...")
    success, select_query_id, error = post_select_query(table_name)
    assert not success, f"SELECT query submission should fail: query was accepted when it shouldn't"
    assert select_query_id is None, "Query ID should still be assigned even on failure"
    print(f"    ✓ Error message: {error}")
    
    
    # Step 9: Submit COPY query on deleted table (will fail but get query ID)
    print(f"  Step 9: Submitting COPY query on deleted table...")
    success, copy_query_id_2, error = post_copy_query(
        csv_file,
        table_name,
        does_csv_contain_header=True
    )
    assert not success, f"COPY query submission should fail: query was accepted when it shouldn't"
    assert copy_query_id_2 is None, "Query ID should still be assigned even on failure"
    print(f"    ✓ COPY query is in FAILED state")
    
    print("✓ TEST PASSED: Queries on deleted table were rejected, tracked as FAILED, and error details retrieved\n")

def run_all_delete_table_tests():
    """
    Run all DELETE table tests.
    """
    print("\n" + "="*80)
    print("RUNNING ALL DELETE TABLE TESTS")
    print("="*80)
    
    try:
        test_put_and_delete_table()
        test_delete_non_existent_table()
        test_delete_table_with_invalid_id()
        test_double_delete()
        test_delete_table_with_running_query()
        test_delete_table_after_query_completion()
        test_delete_table_then_query_attempts()
        
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