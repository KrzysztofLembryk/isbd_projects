#!/usr/bin/env python3
import requests
from typing import Dict, List, Optional, Tuple, Any
from db_client import (get_table_by_id, delete_table, put_table, get_tables, SERVER_URL)

# ============================================================================
# END-TO-END PUT_TABLE TESTS
# ============================================================================

def test_put_table_and_validate_schema():
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
    print("✓ TEST 1 PASSED: Table created and schema validated successfully\n")


def test_put_table_with_empty_name():
    """
    Test 2: Attempt to create a table with an empty name (should fail).
    """
    print("\n" + "="*80)
    print("TEST 2: PUT table with empty name")
    print("="*80)
    
    table_name = ""
    columns = [
        {"name": "id", "type": "INT64"},
        {"name": "name", "type": "VARCHAR"}
    ]
    
    success, table_id, error = put_table(table_name, columns)
    assert not success, "Should fail when table name is empty"
    assert table_id is None, "Table ID should be None on failure"
    assert error is not None, "Error message should be present"
    
    print("✓ TEST 2 PASSED: Empty table name correctly rejected\n")


def test_put_table_with_no_columns():
    """
    Test 3: Attempt to create a table with no columns (should fail).
    """
    print("\n" + "="*80)
    print("TEST 3: PUT table with no columns")
    print("="*80)
    
    table_name = "empty_columns_table"
    columns = []
    
    success, table_id, error = put_table(table_name, columns)
    assert not success, "Should fail when columns list is empty"
    assert table_id is None, "Table ID should be None on failure"
    assert error is not None, "Error message should be present"
    
    print("✓ TEST 3 PASSED: Empty columns list correctly rejected\n")


def test_put_table_with_unsupported_column_type():
    """
    Test 4: Attempt to create a table with unsupported column type (should fail).
    """
    print("\n" + "="*80)
    print("TEST 4: PUT table with unsupported column type")
    print("="*80)
    
    table_name = "invalid_type_table"
    columns = [
        {"name": "id", "type": "INT64"},
        {"name": "data", "type": "BLOB"},  # Unsupported type
        {"name": "timestamp", "type": "DATETIME"}  # Unsupported type
    ]
    
    success, table_id, error = put_table(table_name, columns)
    assert not success, "Should fail when column has unsupported type"
    assert table_id is None, "Table ID should be None on failure"
    assert error is not None, "Error message should be present"


    table_name = "invalid_type_table"
    columns = [
        {"name": "id", "type": "INT64"},
        {"name": "data", "type": "VARCHAR"},  
        {"name": "timestamp", "type": "iNT64"}  # Unsupported type
    ]
    
    success, table_id, error = put_table(table_name, columns)
    assert not success, "Should fail when column has unsupported type"
    assert table_id is None, "Table ID should be None on failure"
    assert error is not None, "Error message should be present"
    
    print("✓ TEST 4 PASSED: Unsupported column type correctly rejected\n")


def test_put_table_with_non_ascii_column_name():
    """
    Test 5: Attempt to create a table with non-ASCII characters in column name (should fail).
    """
    print("\n" + "="*80)
    print("TEST 5: PUT table with non-ASCII column name")
    print("="*80)
    
    table_name = "non_ascii_table"
    columns = [
        {"name": "id", "type": "INT64"},
        {"name": "użytkownik", "type": "VARCHAR"},  # Polish characters
        {"name": "年齢", "type": "INT64"},  # Japanese characters
        {"name": "имя", "type": "VARCHAR"}  # Cyrillic characters
    ]
    
    success, table_id, error = put_table(table_name, columns)
    assert not success, "Should fail when column name contains non-ASCII characters"
    assert table_id is None, "Table ID should be None on failure"
    assert error is not None, "Error message should be present"
    
    print("✓ TEST 5 PASSED: Non-ASCII column names correctly rejected\n")


def test_put_table_with_malformed_payload():
    """
    Test 6: Attempt to create table with various malformed payloads (should fail).
    """
    print("\n" + "="*80)
    print("TEST 6: PUT table with malformed payloads")
    print("="*80)
    
    url = f"{SERVER_URL}/table"
    
    # Test 6a: Wrong "name" key
    payload = {
        "tableName": "test_table",  # Wrong key (should be "name")
        "columns": [{"name": "id", "type": "INT64"}]
    }
    response = requests.put(url, json=payload)
    assert response.status_code == 400, "Should fail with wrong 'name' key"
    print("  ✓ 6a: Wrong 'name' key rejected")
    
    # Test 6b: Wrong "columns" key
    payload = {
        "name": "test_table",
        "cols": [{"name": "id", "type": "INT64"}]  # Wrong key (should be "columns")
    }
    response = requests.put(url, json=payload)
    assert response.status_code == 400, "Should fail with wrong 'columns' key"
    print("  ✓ 6b: Wrong 'columns' key rejected")
    
    # Test 6c: Missing "name" key
    payload = {
        "columns": [{"name": "id", "type": "INT64"}]
    }
    response = requests.put(url, json=payload)
    assert response.status_code == 400, "Should fail with missing 'name' key"
    print("  ✓ 6c: Missing 'name' key rejected")
    
    # Test 6d: Missing "columns" key
    payload = {
        "name": "test_table"
    }
    response = requests.put(url, json=payload)
    assert response.status_code == 400, "Should fail with missing 'columns' key"
    print("  ✓ 6d: Missing 'columns' key rejected")
    
    # Test 6e: Wrong structure in columns list (missing "name")
    payload = {
        "name": "test_table",
        "columns": [{"type": "INT64"}]  # Missing "name"
    }
    response = requests.put(url, json=payload)
    assert response.status_code == 400, "Should fail with missing column 'name'"
    print("  ✓ 6e: Missing column 'name' rejected")
    
    # Test 6f: Wrong structure in columns list (missing "type")
    payload = {
        "name": "test_table",
        "columns": [{"name": "id"}]  # Missing "type"
    }
    response = requests.put(url, json=payload)
    assert response.status_code == 400, "Should fail with missing column 'type'"
    print("  ✓ 6f: Missing column 'type' rejected")
    
    # Test 6g: Columns is not a list
    payload = {
        "name": "test_table",
        "columns": {"name": "id", "type": "INT64"}  # Should be a list
    }
    response = requests.put(url, json=payload)
    assert response.status_code == 400, "Should fail when columns is not a list"
    print("  ✓ 6g: Non-list 'columns' rejected")
    
    # Test 6h: Empty payload
    response = requests.put(url, json={})
    assert response.status_code == 400, "Should fail with empty payload"
    print("  ✓ 6h: Empty payload rejected")
    
    print("✓ TEST 6 PASSED: All malformed payloads correctly rejected\n")


def test_put_table_with_extra_keys():
    """
    Test 7: Attempt to create table with extra keys in payload.
    This should either succeed (ignoring extra keys) or fail (strict validation).
    """
    print("\n" + "="*80)
    print("TEST 7: PUT table with extra keys in payload")
    print("="*80)
    
    table_name = "extra_keys_table"
    
    url = f"{SERVER_URL}/table"
    
    # Test 7a: Extra keys in main payload
    payload = {
        "name": table_name,
        "columns": [
            {"name": "id", "type": "INT64"},
            {"name": "name", "type": "VARCHAR"}
        ],
        "extraKey1": "should be ignored or cause error",
        "extraKey2": 12345
    }
    response = requests.put(url, json=payload)
    
    if response.status_code == 200:
        # Server accepts and ignores extra keys
        table_id = response.json()
        print(f"  ✓ 7a: Server accepted payload with extra keys (lenient mode)")
        delete_table(table_id)
    elif response.status_code == 400:
        # Server rejects extra keys
        print(f"  ✓ 7a: Server rejected payload with extra keys (strict mode)")
    else:
        assert False, f"Unexpected status code: {response.status_code}"
    
    # Test 7b: Extra keys in column definition
    payload = {
        "name": table_name + "_2",
        "columns": [
            {
                "name": "id",
                "type": "INT64",
                "extraColumnKey": "value"
            }
        ]
    }
    response = requests.put(url, json=payload)
    
    if response.status_code == 200:
        # Server accepts and ignores extra keys in columns
        table_id = response.json()
        print(f"  ✓ 7b: Server accepted column with extra keys (lenient mode)")
        delete_table(table_id)
    elif response.status_code == 400:
        # Server rejects extra keys in columns
        print(f"  ✓ 7b: Server rejected column with extra keys (strict mode)")
    else:
        assert False, f"Unexpected status code: {response.status_code}"
    
    print("✓ TEST 7 PASSED: Extra keys handling validated\n")


def test_put_multiple_tables_and_validate_all():
    """
    Test 8: Create 3 tables, validate each individually, and verify /tables returns all of them.
    """
    print("\n" + "="*80)
    print("TEST 8: PUT multiple tables and validate /tables endpoint")
    print("="*80)
    
    # Define 3 different tables
    tables_to_create = [
        {
            "name": "employees",
            "columns": [
                {"name": "emp_id", "type": "INT64"},
                {"name": "name", "type": "VARCHAR"},
                {"name": "salary", "type": "INT64"}
            ]
        },
        {
            "name": "departments",
            "columns": [
                {"name": "dept_id", "type": "INT64"},
                {"name": "dept_name", "type": "VARCHAR"}
            ]
        },
        {
            "name": "projects",
            "columns": [
                {"name": "project_id", "type": "INT64"},
                {"name": "title", "type": "VARCHAR"},
                {"name": "budget", "type": "INT64"},
                {"name": "description", "type": "VARCHAR"}
            ]
        }
    ]
    
    created_table_ids = []
    
    try:
        # Create all 3 tables
        print("\n  Step 1: Creating 3 tables...")
        for table_def in tables_to_create:
            success, table_id, error = put_table(table_def["name"], table_def["columns"])

            assert success, f"Failed to create table '{table_def['name']}': {error}"
            assert table_id is not None, f"Table ID should not be None for '{table_def['name']}'"

            created_table_ids.append(table_id)

            print(f"    ✓ Created table '{table_def['name']}' with ID: {table_id}")
        
        # Validate each table individually with GET /table/{id}
        print("\n  Step 2: Validating each table individually...")
        for i, table_def in enumerate(tables_to_create):
            table_id = created_table_ids[i]
            success, schema, error = get_table_by_id(table_id)
            
            assert success, f"Failed to get schema for table '{table_def['name']}': {error}"
            assert schema is not None, f"Schema should not be None for '{table_def['name']}'"

            assert schema["name"] == table_def["name"], \
                f"Table name mismatch: expected '{table_def['name']}', got '{schema['name']}'"
            assert len(schema["columns"]) == len(table_def["columns"]), \
                f"Column count mismatch for '{table_def['name']}': expected {len(table_def['columns'])}, got {len(schema['columns'])}"
            
            # Validate columns
            for j, col in enumerate(table_def["columns"]):
                returned_col = schema["columns"][j]
                assert returned_col["name"] == col["name"], \
                    f"Column {j} name mismatch in '{table_def['name']}': expected '{col['name']}', got '{returned_col['name']}'"
                assert returned_col["type"] == col["type"], \
                    f"Column {j} type mismatch in '{table_def['name']}': expected '{col['type']}', got '{returned_col['type']}'"
            
            print(f"    ✓ Validated table '{table_def['name']}'")
        
        # Get all tables and verify all 3 are present
        print("\n  Step 3: Verifying /tables endpoint returns all created tables...")
        success, all_tables, error = get_tables()
        assert success, f"Failed to get all tables: {error}"
        assert all_tables is not None, "Tables list should not be None"
        
        # Extract IDs and names from returned tables
        returned_table_ids = {table["tableId"] for table in all_tables}
        returned_table_names = {table["name"] for table in all_tables}
        
        # Verify all created tables are in the response
        for i, table_def in enumerate(tables_to_create):
            table_id = created_table_ids[i]
            table_name = table_def["name"]
            
            assert table_id in returned_table_ids, \
                f"Table ID '{table_id}' for '{table_name}' not found in /tables response"
            assert table_name in returned_table_names, \
                f"Table name '{table_name}' not found in /tables response"
            
            print(f"    ✓ Table '{table_name}' found in /tables response")
        
        print("\n✓ TEST 8 PASSED: All 3 tables created, validated individually, and found in /tables endpoint\n")
        
    finally:
        # Cleanup: Delete all created tables
        print("  Cleanup: Deleting created tables...")
        for table_id in created_table_ids:
            delete_table(table_id)
        print("  ✓ Cleanup complete\n")


def run_all_put_table_tests():
    """
    Run all PUT table tests.
    """
    print("\n" + "="*80)
    print("RUNNING ALL PUT TABLE TESTS")
    print("="*80)
    
    try:
        test_put_table_and_validate_schema()
        test_put_table_with_empty_name()
        test_put_table_with_no_columns()
        test_put_table_with_unsupported_column_type()
        test_put_table_with_non_ascii_column_name()
        test_put_table_with_malformed_payload()
        test_put_table_with_extra_keys()
        test_put_multiple_tables_and_validate_all()
        
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
    run_all_put_table_tests()