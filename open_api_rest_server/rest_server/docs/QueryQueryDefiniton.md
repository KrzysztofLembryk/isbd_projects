# QueryQueryDefiniton

## Properties
Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**table_name** | **String** |  | [optional] [default to None]
**source_filepath** | **String** | Path to source CSV file (filepath in perspective of running server! NOT client) | 
**destination_table_name** | **String** |  | 
**destination_columns** | **Vec<String>** | List of columns to copy data into. It creates a map from source columns to destination columns. Assumes that data in source file is in the same order as in this list. | [optional] [default to None]
**does_csv_contain_header** | **bool** | Whether CSV file contains header row | [optional] [default to Some(false)]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


