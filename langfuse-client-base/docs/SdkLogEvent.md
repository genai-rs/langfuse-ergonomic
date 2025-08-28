# SdkLogEvent

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**body** | [**models::SdkLogBody**](SDKLogBody.md) |  | 
**id** | **String** | UUID v4 that identifies the event | 
**timestamp** | **String** | Datetime (ISO 8601) of event creation in client. Should be as close to actual event creation in client as possible, this timestamp will be used for ordering of events in future release. Resolution: milliseconds (required), microseconds (optimal). | 
**metadata** | Option<[**serde_json::Value**](.md)> | Optional. Metadata field used by the Langfuse SDKs for debugging. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


