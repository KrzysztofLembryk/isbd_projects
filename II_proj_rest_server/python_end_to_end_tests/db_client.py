#!/usr/bin/env python3
import requests
from typing import Dict, List, Optional, Tuple, Any

ERROR_KEY = "message"
SERVER_URL = "http://localhost:8080"

# ============================================================================
# TABLE SCHEMA ENDPOINTS
# ============================================================================

def get_tables() -> Tuple[bool, Optional[List[Dict]], Optional[str]]:
    """
    Get list of tables with their accompanying IDs.
    
    Returns:
        tuple: (success: bool, tables: List[Dict] or None, error_msg: str or None)
    """
    url = f"{SERVER_URL}/tables"
    
    try:
        response = requests.get(url)
        
        if response.status_code == 200:
            tables = response.json()
            print(f"✓ Retrieved {len(tables)} tables")
            return True, tables, None
        else:
            error_msg = response.json().get(ERROR_KEY, "Unknown error")
            print(f"✗ Failed to get tables: {error_msg}")
            return False, None, error_msg
            
    except Exception as e:
        print(f"✗ Error getting tables: {e}")
        return False, None, str(e)

def get_table_by_id(table_id: str) -> Tuple[bool, Optional[Dict], Optional[str]]:
    """
    Get detailed description of selected table.
    
    Args:
        table_id: ID of the table
    
    Returns:
        tuple: (success: bool, table_schema: Dict or None, error_msg: str or None)
    """
    url = f"{SERVER_URL}/table/{table_id}"
    
    try:
        response = requests.get(url)
        
        if response.status_code == 200:
            table_schema = response.json()
            print(f"✓ Retrieved table schema for ID: {table_id}")
            return True, table_schema, None
        elif response.status_code == 404:
            error_msg = response.json().get(ERROR_KEY, "Table not found")
            print(f"✗ Table not found: {error_msg}")
            return False, None, error_msg
        else:
            error_msg = response.json().get(ERROR_KEY, "Unknown error")
            print(f"✗ Failed to get table: {error_msg}")
            return False, None, error_msg
            
    except Exception as e:
        print(f"✗ Error getting table: {e}")
        return False, None, str(e)

def delete_table(table_id: str) -> Tuple[bool, Optional[str]]:
    """
    Delete selected table from database.
    
    Args:
        table_id: ID of the table to delete
    
    Returns:
        tuple: (success: bool, error_msg: str or None)
    """
    url = f"{SERVER_URL}/table/{table_id}"
    
    try:
        response = requests.delete(url)
        
        if response.status_code == 200:
            print(f"✓ Table {table_id} deleted successfully")
            return True, None
        elif response.status_code == 404:
            error_msg = response.json().get(ERROR_KEY, "Table not found")
            print(f"✗ Table not found: {error_msg}")
            return False, error_msg
        else:
            error_msg = response.json().get(ERROR_KEY, "Unknown error")
            print(f"✗ Failed to delete table: {error_msg}")
            return False, error_msg
            
    except Exception as e:
        print(f"✗ Error deleting table: {e}")
        return False, str(e)

def put_table(table_name: str, columns: List[Dict]) -> Tuple[bool, Optional[str], Optional[str]]:
    """
    Create new table in database.
    
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
            table_id = response.json()
            print(f"✓ Table '{table_name}' created successfully with ID: {table_id}")
            return True, table_id, None
        elif response.status_code == 400:
            error_msg = response.json().get(ERROR_KEY, [{"error": "Unknown error"}])
            print(f"✗ Failed to create table '{table_name}': {error_msg}")
            return False, None, str(error_msg)
        elif response.status_code == 404:
            error_msg = response.json().get(ERROR_KEY, [{"error": "Unknown error"}])
            print(f"✗ Failed to create table '{table_name}': BAD REQUEST {error_msg}")
            return False, None, str(error_msg)
        else:
            error_msg = response.json().get(ERROR_KEY, "Unknown error")
            print(f"✗ Failed to create table '{table_name}': {error_msg}")
            return False, None, error_msg
            
    except Exception as e:
        print(f"✗ Error creating table '{table_name}': {e}")
        return False, None, str(e)


# ============================================================================
# QUERIES ENDPOINTS
# ============================================================================

def get_queries() -> Tuple[bool, Optional[List[Dict]], Optional[str]]:
    """
    Get list of queries.
    
    Returns:
        tuple: (success: bool, queries: List[Dict] or None, error_msg: str or None)
    """
    url = f"{SERVER_URL}/queries"
    
    try:
        response = requests.get(url)
        
        if response.status_code == 200:
            queries = response.json()
            print(f"✓ Retrieved {len(queries)} queries")
            return True, queries, None
        else:
            error_msg = response.json().get(ERROR_KEY, "Unknown error")
            print(f"✗ Failed to get queries: {error_msg}")
            return False, None, error_msg
            
    except Exception as e:
        print(f"✗ Error getting queries: {e}")
        return False, None, str(e)


def get_query_by_id(query_id: str) -> Tuple[bool, Optional[Dict], Optional[str]]:
    """
    Get detailed status of selected query.
    
    Args:
        query_id: ID of the query
    
    Returns:
        tuple: (success: bool, query: Dict or None, error_msg: str or None)
    """
    url = f"{SERVER_URL}/query/{query_id}"
    
    try:
        response = requests.get(url)
        
        if response.status_code == 200:
            query = response.json()
            print(f"✓ Retrieved query status for ID: {query_id}")
            return True, query, None
        elif response.status_code == 404:
            error_msg = response.json().get(ERROR_KEY, "Query not found")
            print(f"✗ Query not found: {error_msg}")
            return False, None, error_msg
        else:
            error_msg = response.json().get(ERROR_KEY, "Unknown error")
            print(f"✗ Failed to get query: {error_msg}")
            return False, None, error_msg
            
    except Exception as e:
        print(f"✗ Error getting query: {e}")
        return False, None, str(e)

def post_query(query_definition: Dict) -> Tuple[bool, Optional[str], Optional[str]]:
    """
    Submit new query for execution.
    
    Args:
        query_definition: Dict with query details
                         e.g., {"queryDefinition": {"tableName": "table1"}} for SELECT
                         or {"queryDefinition": {"sourceFilepath": "...", ...}} for COPY
    
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
        elif response.status_code == 400:
            error_msg = response.json().get("problems", [{"error": "Unknown error"}])
            print(f"✗ Failed to submit query: {error_msg}")
            return False, None, str(error_msg)
        else:
            error_msg = response.json().get("message", "Unknown error")
            print(f"✗ Failed to submit query: {error_msg}")
            return False, None, error_msg
            
    except Exception as e:
        print(f"✗ Error submitting query: {e}")
        return False, None, str(e)

def get_failed_query(query_id: str) -> Tuple[bool, Optional[Dict], Optional[str]]:
    """
    Get error details of a failed query.
    
    Args:
        query_id: ID of the failed query
    
    Returns:
        tuple: (success: bool, error_details: Dict or None, error_msg: str or None)
    """
    url = f"{SERVER_URL}/error/{query_id}"
    
    try:
        response = requests.get(url)
        
        if response.status_code == 200:
            error_details = response.json()
            print(f"✓ Retrieved error details for query ID: {query_id}")
            return True, error_details, None
        elif response.status_code == 404:
            error_msg = response.json().get(ERROR_KEY, "Query not found")
            print(f"✗ Query not found: {error_msg}")
            return False, None, error_msg
        elif response.status_code == 400:
            error_msg = response.json().get(ERROR_KEY, "Query is not in FAILED state")
            print(f"✗ Error details not available: {error_msg}")
            return False, None, error_msg
        else:
            error_msg = response.json().get(ERROR_KEY, "Unknown error")
            print(f"✗ Failed to get error details: {error_msg}")
            return False, None, error_msg
            
    except Exception as e:
        print(f"✗ Error getting failed query details: {e}")
        return False, None, str(e)

def get_query_result(
        query_id: str, 
        row_limit: Optional[int] = None, 
        flush_result: Optional[bool] = None
        ) -> Tuple[bool, Optional[Dict], Optional[str]]:
    """
    Get result of selected query (only for SELECT queries after completion).
    
    Args:
        query_id: ID of the query
        row_limit: Maximum number of rows to return (optional)
    
    Returns:
        tuple: (success: bool, result: Dict or None, error_msg: str or None)
    """
    url = f"{SERVER_URL}/result/{query_id}"
    
    payload = {}
    if row_limit is not None:
        payload["rowLimit"] = row_limit
    
    if flush_result is not None:
        payload["flushResult"] = flush_result
    
    try:
        response = requests.get(url, json=payload if payload else None)
        
        if response.status_code == 200:
            result = response.json()
            print(f"✓ Retrieved query result for ID: {query_id}")
            return True, result, None
        elif response.status_code == 404:
            error_msg = response.json().get(ERROR_KEY, "Query not found")
            print(f"✗ Query not found: {error_msg}")
            return False, None, error_msg
        elif response.status_code == 400:
            error_msg = response.json().get(ERROR_KEY, "Result not available")
            print(f"✗ Result not available: {error_msg}")
            return False, None, error_msg
        else:
            error_msg = response.json().get(ERROR_KEY, "Unknown error")
            print(f"✗ Failed to get query result: {error_msg}")
            return False, None, error_msg
            
    except Exception as e:
        print(f"✗ Error getting query result: {e}")
        return False, None, str(e)


def get_query_error(query_id: str) -> Tuple[bool, Optional[Dict], Optional[str]]:
    """
    Get error of selected query (only for queries in FAILED state).
    
    Args:
        query_id: ID of the query
    
    Returns:
        tuple: (success: bool, error_details: Dict or None, error_msg: str or None)
    """
    url = f"{SERVER_URL}/error/{query_id}"
    
    try:
        response = requests.get(url)
        
        if response.status_code == 200:
            error_details = response.json()
            print(f"✓ Retrieved query error for ID: {query_id}")
            return True, error_details, None
        elif response.status_code == 404:
            error_msg = response.json().get(ERROR_KEY, "Query not found")
            print(f"✗ Query not found: {error_msg}")
            return False, None, error_msg
        elif response.status_code == 400:
            error_msg = response.json().get(ERROR_KEY, "Error not available")
            print(f"✗ Error not available: {error_msg}")
            return False, None, error_msg
        else:
            error_msg = response.json().get(ERROR_KEY, "Unknown error")
            print(f"✗ Failed to get query error: {error_msg}")
            return False, None, error_msg
            
    except Exception as e:
        print(f"✗ Error getting query error: {e}")
        return False, None, str(e)


# ============================================================================
# METADATA ENDPOINTS
# ============================================================================

def get_system_info() -> Tuple[bool, Optional[Dict], Optional[str]]:
    """
    Get basic information about the system.
    
    Returns:
        tuple: (success: bool, system_info: Dict or None, error_msg: str or None)
    """
    url = f"{SERVER_URL}/system/info"
    
    try:
        response = requests.get(url)
        
        if response.status_code == 200:
            system_info = response.json()
            print(f"✓ Retrieved system information")
            return True, system_info, None
        else:
            error_msg = response.json().get("message", "Unknown error")
            print(f"✗ Failed to get system info: {error_msg}")
            return False, None, error_msg
            
    except Exception as e:
        print(f"✗ Error getting system info: {e}")
        return False, None, str(e)


# ============================================================================
# HELPER FUNCTIONS (Specific Query Types)
# ============================================================================

def post_select_query(table_name: str) -> Tuple[bool, Optional[str], Optional[str]]:
    """
    Submit a SELECT query.
    
    Args:
        table_name: Name of the table to select from
    
    Returns:
        tuple: (success: bool, query_id: str or None, error_msg: str or None)
    """
    query_def = {
        "queryDefinition": {
            "tableName": table_name
        }
    }
    
    return post_query(query_def)


def post_copy_query(
    source_filepath: str,
    destination_table_name: str,
    destination_columns: Optional[List[str]] = None,
    does_csv_contain_header: bool = False
) -> Tuple[bool, Optional[str], Optional[str]]:
    """
    Submit a COPY query.
    
    Args:
        source_filepath: Path to source CSV file (from server's perspective)
        destination_table_name: Name of the destination table
        destination_columns: Optional list of column names for mapping
        does_csv_contain_header: Whether CSV file contains a header row
    
    Returns:
        tuple: (success: bool, query_id: str or None, error_msg: str or None)
    """
    query_def = {
        "queryDefinition": {
            "sourceFilepath": source_filepath,
            "destinationTableName": destination_table_name,
            "doesCsvContainHeader": does_csv_contain_header
        }
    }
    
    if destination_columns is not None:
        query_def["queryDefinition"]["destinationColumns"] = destination_columns
    
    return post_query(query_def)



# needed to remove data from db after given test

# ============================================================================
# END-TO-END TESTS
# ============================================================================

