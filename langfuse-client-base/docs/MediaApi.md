# \MediaApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**media_get**](MediaApi.md#media_get) | **GET** /api/public/media/{mediaId} | 
[**media_get_upload_url**](MediaApi.md#media_get_upload_url) | **POST** /api/public/media | 
[**media_patch**](MediaApi.md#media_patch) | **PATCH** /api/public/media/{mediaId} | 



## media_get

> models::GetMediaResponse media_get(media_id)


Get a media record

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**media_id** | **String** | The unique langfuse identifier of a media record | [required] |

### Return type

[**models::GetMediaResponse**](GetMediaResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## media_get_upload_url

> models::GetMediaUploadUrlResponse media_get_upload_url(get_media_upload_url_request)


Get a presigned upload URL for a media record

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**get_media_upload_url_request** | [**GetMediaUploadUrlRequest**](GetMediaUploadUrlRequest.md) |  | [required] |

### Return type

[**models::GetMediaUploadUrlResponse**](GetMediaUploadUrlResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## media_patch

> media_patch(media_id, patch_media_body)


Patch a media record

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**media_id** | **String** | The unique langfuse identifier of a media record | [required] |
**patch_media_body** | [**PatchMediaBody**](PatchMediaBody.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

