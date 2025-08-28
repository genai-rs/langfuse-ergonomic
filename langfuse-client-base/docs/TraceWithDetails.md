# TraceWithDetails

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**html_path** | **String** | Path of trace in Langfuse UI | 
**latency** | **f64** | Latency of trace in seconds | 
**total_cost** | **f64** | Cost of trace in USD | 
**observations** | **Vec<String>** | List of observation ids | 
**scores** | **Vec<String>** | List of score ids | 
**id** | **String** | The unique identifier of a trace | 
**timestamp** | **String** | The timestamp when the trace was created | 
**name** | Option<**String**> | The name of the trace | [optional]
**input** | Option<[**serde_json::Value**](.md)> | The input data of the trace. Can be any JSON. | [optional]
**output** | Option<[**serde_json::Value**](.md)> | The output data of the trace. Can be any JSON. | [optional]
**session_id** | Option<**String**> | The session identifier associated with the trace | [optional]
**release** | Option<**String**> | The release version of the application when the trace was created | [optional]
**version** | Option<**String**> | The version of the trace | [optional]
**user_id** | Option<**String**> | The user identifier associated with the trace | [optional]
**metadata** | Option<[**serde_json::Value**](.md)> | The metadata associated with the trace. Can be any JSON. | [optional]
**tags** | Option<**Vec<String>**> | The tags associated with the trace. Can be an array of strings or null. | [optional]
**public** | Option<**bool**> | Public traces are accessible via url without login | [optional]
**environment** | Option<**String**> | The environment from which this trace originated. Can be any lowercase alphanumeric string with hyphens and underscores that does not start with 'langfuse'. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


