# GetMediaUploadUrlRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**trace_id** | **String** | The trace ID associated with the media record | 
**observation_id** | Option<**String**> | The observation ID associated with the media record. If the media record is associated directly with a trace, this will be null. | [optional]
**content_type** | [**models::MediaContentType**](MediaContentType.md) |  | 
**content_length** | **i32** | The size of the media record in bytes | 
**sha256_hash** | **String** | The SHA-256 hash of the media record | 
**field** | **String** | The trace / observation field the media record is associated with. This can be one of `input`, `output`, `metadata` | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


