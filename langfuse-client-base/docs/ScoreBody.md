# ScoreBody

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**String**> |  | [optional]
**trace_id** | Option<**String**> |  | [optional]
**session_id** | Option<**String**> |  | [optional]
**observation_id** | Option<**String**> |  | [optional]
**dataset_run_id** | Option<**String**> |  | [optional]
**name** | **String** |  | 
**environment** | Option<**String**> |  | [optional]
**value** | [**models::CreateScoreValue**](CreateScoreValue.md) |  | 
**comment** | Option<**String**> |  | [optional]
**metadata** | Option<[**serde_json::Value**](.md)> |  | [optional]
**data_type** | Option<[**models::ScoreDataType**](ScoreDataType.md)> |  | [optional]
**config_id** | Option<**String**> | Reference a score config on a score. When set, the score name must equal the config name and scores must comply with the config's range and data type. For categorical scores, the value must map to a config category. Numeric scores might be constrained by the score config's max and min values | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


