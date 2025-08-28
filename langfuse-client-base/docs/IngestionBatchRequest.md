# IngestionBatchRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**batch** | [**Vec<models::IngestionEvent>**](IngestionEvent.md) | Batch of tracing events to be ingested. Discriminated by attribute `type`. | 
**metadata** | Option<[**serde_json::Value**](.md)> | Optional. Metadata field used by the Langfuse SDKs for debugging. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


