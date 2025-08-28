# \ScoreApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**score_create**](ScoreApi.md#score_create) | **POST** /api/public/scores | 
[**score_delete**](ScoreApi.md#score_delete) | **DELETE** /api/public/scores/{scoreId} | 



## score_create

> models::CreateScoreResponse score_create(create_score_request)


Create a score (supports both trace and session scores)

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_score_request** | [**CreateScoreRequest**](CreateScoreRequest.md) |  | [required] |

### Return type

[**models::CreateScoreResponse**](CreateScoreResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## score_delete

> score_delete(score_id)


Delete a score (supports both trace and session scores)

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**score_id** | **String** | The unique langfuse identifier of a score | [required] |

### Return type

 (empty response body)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

