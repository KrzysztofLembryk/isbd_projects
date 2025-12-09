#!/usr/bin/env python3
from db_client import (get_table_by_id, delete_table, put_table)

def test_put_table_and_get_its_details():
    """
    Test 1: Create a table and validate that GET returns correct schema.
    """
    print("\n" + "="*80)
    print("TEST 1: PUT table and validate schema with GET")
    print("="*80)
    
    table_name = "test_users_table"
    columns = [
        {"name": "id", "type": "INT64"},
        {"name": "username", "type": "VARCHAR"},
        {"name": "age", "type": "INT64"},
        {"name": "email", "type": "VARCHAR"}
    ]
    
    # Create table
    success, table_id, error = put_table(table_name, columns)
    assert success, f"Failed to create table: {error}"
    assert table_id is not None, "Table ID should not be None"
    
    # Validate schema with GET
    success, schema, error = get_table_by_id(table_id)
    assert success, f"Failed to get table schema: {error}"
    assert schema is not None, "Schema should not be None"
    assert schema["name"] == table_name, f"Table name mismatch: expected '{table_name}', got '{schema['name']}'"
    assert len(schema["columns"]) == len(columns), f"Column count mismatch: expected {len(columns)}, got {len(schema['columns'])}"
    
    # Validate each column
    for i, col in enumerate(columns):
        returned_col = schema["columns"][i]
        assert returned_col["name"] == col["name"], f"Column {i} name mismatch: expected '{col['name']}', got '{returned_col['name']}'"
        assert returned_col["type"] == col["type"], f"Column {i} type mismatch: expected '{col['type']}', got '{returned_col['type']}'"
    
    # Cleanup
    delete_table(table_id)
    print("✓ TEST 1 PASSED: Table created and getting it details was successful\n")

def test_non_existent_id():
    """
    Test 2: Try to get details of a table with non-existent ID (should fail with 404).
    """
    print("\n" + "="*80)
    print("TEST 2: GET table details with non-existent ID")
    print("="*80)
    
    non_existent_id = "00000000-0000-0000-0000-000000000001"
    
    print(f"  Requesting table with non-existent ID: {non_existent_id}")
    
    # Try to get table details
    success, schema, error = get_table_by_id(non_existent_id)
    
    assert not success, "Request should fail for non-existent table ID"
    assert schema is None, "Schema should be None for non-existent table"
    assert error is not None, "Error message should be present"
    
    print(f"  ✓ Correctly received error: {error}")
    print("✓ TEST 2 PASSED: Non-existent table ID correctly rejected with 404\n")
    
def test_id_not_uuid():
    """
    Test 3: Try to get details of a table with invalid ID format (not a UUID).
    """
    print("\n" + "="*80)
    print("TEST 3: GET table details with invalid ID format (not UUID)")
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
        print(f"  Requesting table with invalid ID: {invalid_id}")
        
        # Try to get table details
        success, schema, error = get_table_by_id(invalid_id)
        
        assert not success, f"Request should fail for invalid ID format: {invalid_id}"
        assert schema is None, f"Schema should be None for invalid ID: {invalid_id}"
        assert error is not None, f"Error message should be present for invalid ID: {invalid_id}"
        
        print(f"    ✓ Correctly received error: {error}")
    
    print("✓ TEST 3 PASSED: Invalid ID formats correctly rejected\n")


def run_all_get_table_details_tests():
    """
    Run all PUT table tests.
    """
    print("\n" + "="*80)
    print("RUNNING ALL GET TABLE DETAILS TESTS")
    print("="*80)
    
    try:
        test_put_table_and_get_its_details()
        test_non_existent_id()
        test_id_not_uuid()
        
        print("\n" + "="*80)
        print("ALL TESTS PASSED! ✓")
        print("="*80 + "\n")
        
    except AssertionError as e:
        print(f"\n✗ TEST FAILED: {e}\n")
        raise
    except Exception as e:
        print(f"\n✗ UNEXPECTED ERROR: {e}\n")
        raise


# ============================================================================
# MAIN
# ============================================================================

if __name__ == "__main__":
    # Run all tests
    run_all_get_table_details_tests()