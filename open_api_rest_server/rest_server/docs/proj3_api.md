# proj3_api

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
**submitQuery**](proj3_api.md#submitQuery) | **POST** /query | Submit new query for execution
**getQueryById**](proj3_api.md#getQueryById) | **GET** /query/{queryId} | Get detailed status of selected query
**getQueryError**](proj3_api.md#getQueryError) | **GET** /error/{queryId} | Get error of selected query (will be available only for queries in FAILED state)
**getQueryResult**](proj3_api.md#getQueryResult) | **GET** /result/{queryId} | Get result of selected query (will be available only for SELECT queries after they are completed)


# **submitQuery**
> String submitQuery(execute_query_request)
Submit new query for execution

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **execute_query_request** | [**ExecuteQueryRequest**](ExecuteQueryRequest.md)| Used to submit a new query for execution | 

### Return type

[**String**](string.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getQueryById**
> models::Query getQueryById(query_id)
Get detailed status of selected query

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **query_id** | **String**| ID of selected Query | 

### Return type

[**models::Query**](Query.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getQueryError**
> models::MultipleProblemsError getQueryError(query_id)
Get error of selected query (will be available only for queries in FAILED state)

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **query_id** | **String**| ID of selected Query | 

### Return type

[**models::MultipleProblemsError**](MultipleProblemsError.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **getQueryResult**
> Vec<models::QueryResultInner> getQueryResult(query_id, optional)
Get result of selected query (will be available only for SELECT queries after they are completed)

### Required Parameters

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
  **query_id** | **String**| ID of selected Query | 
 **optional** | **map[string]interface{}** | optional parameters | nil if no parameters

### Optional Parameters
Optional parameters are passed through a map[string]interface{}.

Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **query_id** | **String**| ID of selected Query | 
 **get_query_result_request** | [**GetQueryResultRequest**](GetQueryResultRequest.md)| Used to get result of a query | 

### Return type

[**Vec<models::QueryResultInner>**](QueryResult_inner.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

