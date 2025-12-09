#!/usr/bin/env python3
import requests
import concurrent.futures
import time

SERVER_URL = "http://localhost:8080"
TABLE_ID = "f0652708-9ca9-4d6f-bf34-a52274d1ad16"
NUM_REQUESTS = 1

def get_table_details(request_id: int, table_id: str):
    url = f"{SERVER_URL}/table/{table_id}"
    # url = f"{SERVER_URL}/tables"
    start = time.time()
    try:
        response = requests.get(url)
        duration = time.time() - start
        print(f"Request {request_id} - Status: {response.status_code} - Time: {duration:.3f}s, RESPONSE MSG: {response.content}")
        return response.status_code
    except Exception as e:
        print(f"Request {request_id} - Error: {e}")
        return None

def create_table(table_name, columns):
    """
    Send a PUT request to create a new table.
    
    Args:
        table_name: Name of the table to create
        columns: List of dicts with 'name' and 'type' keys
                 e.g., [{"name": "id", "type": "INT64"}, {"name": "name", "type": "VARCHAR"}]
    
    Returns:
        tuple: (success: bool, table_id: str or None, error_msg: str or None)
    """
    url = f"{SERVER_URL}/table"
    
    payload = {
        "name": table_name,
        "columns": columns
    }
    
    try:
        response = requests.put(url, json=payload)
        
        if response.status_code == 200:
            table_id = response.text
            print(f"✓ Table '{table_name}' created successfully with ID: {table_id}")
            return True, table_id, None
        else:
            error_msg = response.json().get("message", "Unknown error")
            print(f"✗ Failed to create table '{table_name}': {error_msg}")
            return False, None, error_msg
            
    except Exception as e:
        print(f"✗ Error creating table '{table_name}': {e}")
        return False, None, str(e)

# Example usage:
def sample_create_table(table_name: str):
    success, table_id, error = create_table(
        table_name,
        [
            {"name": "id", "type": "INT64"},
            {"name": "username", "type": "VARCHAR"},
            {"name": "age", "type": "INT64"}
        ]
    )
    
    if success:
        print(f"Table created with ID: {table_id}")
    else:
        print(f"CREATE TABLE ERROR: {error}")


def post_query(query_definition):
    """
    Send a POST request to execute a query.
    
    Args:
        query_definition: Dict with query details
                         e.g., {"queryDefinition": {"SelectQ": {...}}} 
                         or {"queryDefinition": {"CopyQ": {...}}}
    
    Returns:
        tuple: (success: bool, query_id: str or None, error_msg: str or None)
    """
    url = f"{SERVER_URL}/query"
    
    try:
        response = requests.post(url, json=query_definition)
        
        if response.status_code == 200:
            query_id = response.json()
            print(f"✓ Query submitted successfully with ID: {query_id}")
            return True, query_id, None
        else:
            error_msg = response.json().get("message", "Unknown error")
            print(f"✗ Failed to submit query: {error_msg}")
            return False, None, error_msg
            
    except Exception as e:
        print(f"✗ Error submitting query: {e}")
        return False, None, str(e)


def sample_post_select_query(table_name):
    """Example: Submit a SELECT query"""
    query_def = {
        "queryDefinition": {
                "tableName": table_name
        }
    }
    
    success, query_id, error = post_query(query_def)
    
    if success:
        print(f"SELECT query created with ID: {query_id}")
    else:
        print(f"POST QUERY ERROR: {error}")


def sample_post_copy_query(table_name, filepath):
    """Example: Submit a COPY query"""
    query_def = {
        "queryDefinition": {
                "sourceFilepath": filepath,
                "destinationTableName": table_name,
                "destinationColumns": None,  # or ["col1", "col2"]
                "doesCsvContainHeader": True
        }
    }
    
    success, query_id, error = post_query(query_def)
    
    if success:
        print(f"COPY query created with ID: {query_id}")
    else:
        print(f"POST QUERY ERROR: {error}")

def post_select_query(table_name):
    """Example: Submit a SELECT query"""
    query_def = {
        "queryDefinition": {
            "tableName": table_name
        }
    }
    
    success, query_id, error = post_query(query_def)
    
    if success:
        print(f"SELECT query created with ID: {query_id}")
    else:
        print(f"POST QUERY ERROR: {error}")

if __name__ == "__main__":
    # sample_get_table_details()
    # for i in range(1, 2):
    #     sample_create_table(f"table_{i}")
    get_table_details(1, TABLE_ID) 
    post_select_query("table_1")
    # sample_post_select_query("table_1")
    # sample_post_select_query("table_X")
    # sample_post_copy_query("table_1", "random/filepath")