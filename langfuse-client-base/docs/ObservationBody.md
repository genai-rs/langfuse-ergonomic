# ObservationBody

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | Option<**String**> |  | [optional]
**trace_id** | Option<**String**> |  | [optional]
**r#type** | [**models::ObservationType**](ObservationType.md) |  | 
**name** | Option<**String**> |  | [optional]
**start_time** | Option<**String**> |  | [optional]
**end_time** | Option<**String**> |  | [optional]
**completion_start_time** | Option<**String**> |  | [optional]
**model** | Option<**String**> |  | [optional]
**model_parameters** | Option<[**std::collections::HashMap<String, models::MapValue>**](MapValue.md)> |  | [optional]
**input** | Option<[**serde_json::Value**](.md)> |  | [optional]
**version** | Option<**String**> |  | [optional]
**metadata** | Option<[**serde_json::Value**](.md)> |  | [optional]
**output** | Option<[**serde_json::Value**](.md)> |  | [optional]
**usage** | Option<[**models::Usage**](Usage.md)> |  | [optional]
**level** | Option<[**models::ObservationLevel**](ObservationLevel.md)> |  | [optional]
**status_message** | Option<**String**> |  | [optional]
**parent_observation_id** | Option<**String**> |  | [optional]
**environment** | Option<**String**> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


