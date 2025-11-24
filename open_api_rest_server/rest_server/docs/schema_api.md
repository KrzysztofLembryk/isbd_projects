# schema_api

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
**createTable**](schema_api.md#createTable) | **PUT** /table | Create new table in database
**getTables**](schema_api.md#getTables) | **GET** /tables | Get list of tables with their accompaning IDs. Use those IDs to get details by calling /table endpoint.
**deleteTable**](schema_api.md#deleteTable) | **DELETE** /table/{tableId} | Delete selected table from database
**getTableById**](schema_api.md#getTableById) | **GET** /table/{tableId} | Get detailed description of selected table


# **createTable**
> String createTable(table_schema)
Create new table in database

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **table_schema** | [**TableSchema**](TableSchema.md)| Used to create a new table | 

### Return type

[**String**](string.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getTables**
> Vec<models::ShallowTable> getTables()
Get list of tables with their accompaning IDs. Use those IDs to get details by calling /table endpoint.

### Required Parameters
This endpoint does not need any parameter.

### Return type

[**Vec<models::ShallowTable>**](ShallowTable.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **deleteTable**
> deleteTable(table_id)
Delete selected table from database

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **table_id** | **String**| ID of selected Table | 

### Return type

 (empty response body)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getTableById**
> models::TableSchema getTableById(table_id)
Get detailed description of selected table

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **table_id** | **String**| ID of selected Table | 

### Return type

[**models::TableSchema**](TableSchema.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

