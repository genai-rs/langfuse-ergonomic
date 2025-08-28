# CreateDatasetItemRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**dataset_name** | **String** |  | 
**input** | Option<[**serde_json::Value**](.md)> |  | [optional]
**expected_output** | Option<[**serde_json::Value**](.md)> |  | [optional]
**metadata** | Option<[**serde_json::Value**](.md)> |  | [optional]
**source_trace_id** | Option<**String**> |  | [optional]
**source_observation_id** | Option<**String**> |  | [optional]
**id** | Option<**String**> | Dataset items are upserted on their id. Id needs to be unique (project-level) and cannot be reused across datasets. | [optional]
**status** | Option<[**models::DatasetStatus**](DatasetStatus.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


