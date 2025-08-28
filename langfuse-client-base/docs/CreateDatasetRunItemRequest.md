# CreateDatasetRunItemRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**run_name** | **String** |  | 
**run_description** | Option<**String**> | Description of the run. If run exists, description will be updated. | [optional]
**metadata** | Option<[**serde_json::Value**](.md)> | Metadata of the dataset run, updates run if run already exists | [optional]
**dataset_item_id** | **String** |  | 
**observation_id** | Option<**String**> |  | [optional]
**trace_id** | Option<**String**> | traceId should always be provided. For compatibility with older SDK versions it can also be inferred from the provided observationId. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


