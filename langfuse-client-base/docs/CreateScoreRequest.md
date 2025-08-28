# CreateScoreRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**String**> |  | [optional]
**trace_id** | Option<**String**> |  | [optional]
**session_id** | Option<**String**> |  | [optional]
**observation_id** | Option<**String**> |  | [optional]
**dataset_run_id** | Option<**String**> |  | [optional]
**name** | **String** |  | 
**value** | [**models::CreateScoreValue**](CreateScoreValue.md) |  | 
**comment** | Option<**String**> |  | [optional]
**metadata** | Option<[**serde_json::Value**](.md)> |  | [optional]
**environment** | Option<**String**> | The environment of the score. Can be any lowercase alphanumeric string with hyphens and underscores that does not start with 'langfuse'. | [optional]
**data_type** | Option<[**models::ScoreDataType**](ScoreDataType.md)> |  | [optional]
**config_id** | Option<**String**> | Reference a score config on a score. The unique langfuse identifier of a score config. When passing this field, the dataType and stringValue fields are automatically populated. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


