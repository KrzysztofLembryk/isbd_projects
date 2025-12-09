from db_client import (get_table_by_id, delete_table, put_table)

from db_client import (get_table_by_id, delete_table, put_table)


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