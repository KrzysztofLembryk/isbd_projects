#!/usr/bin/env python3
import requests
import concurrent.futures
import time

SERVER_URL = "http://localhost:8080"
TABLE_ID = "8b689f56-15dd-4803-9232-8b1e9ac65269"
NUM_REQUESTS = 10

def send_request(request_id):
    # url = f"{SERVER_URL}/table/{TABLE_ID}"
    url = f"{SERVER_URL}/tables"
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


def sample_get_table_details():
    print(f"Sending {NUM_REQUESTS} concurrent requests...")
    
    with concurrent.futures.ThreadPoolExecutor(max_workers=NUM_REQUESTS) as executor:
        futures = [executor.submit(send_request, i) for i in range(1, NUM_REQUESTS + 1)]
        results = [f.result() for f in concurrent.futures.as_completed(futures)]
    
    print(f"\nAll {NUM_REQUESTS} requests completed")
    print(f"Success: {sum(1 for r in results if r == 200)}")
    print(f"Errors: {sum(1 for r in results if r != 200)}")


if __name__ == "__main__":
    sample_get_table_details()
    # for i in range(1, 20):
    #     sample_create_table(f"table_{i}")