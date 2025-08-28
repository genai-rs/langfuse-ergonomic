# \TraceApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**trace_delete**](TraceApi.md#trace_delete) | **DELETE** /api/public/traces/{traceId} | 
[**trace_delete_multiple**](TraceApi.md#trace_delete_multiple) | **DELETE** /api/public/traces | 
[**trace_get**](TraceApi.md#trace_get) | **GET** /api/public/traces/{traceId} | 
[**trace_list**](TraceApi.md#trace_list) | **GET** /api/public/traces | 



## trace_delete

> models::DeleteTraceResponse trace_delete(trace_id)


Delete a specific trace

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**trace_id** | **String** | The unique langfuse identifier of the trace to delete | [required] |

### Return type

[**models::DeleteTraceResponse**](DeleteTraceResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## trace_delete_multiple

> models::DeleteTraceResponse trace_delete_multiple(trace_delete_multiple_request)


Delete multiple traces

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**trace_delete_multiple_request** | [**TraceDeleteMultipleRequest**](TraceDeleteMultipleRequest.md) |  | [required] |

### Return type

[**models::DeleteTraceResponse**](DeleteTraceResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## trace_get

> models::TraceWithFullDetails trace_get(trace_id)


Get a specific trace

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**trace_id** | **String** | The unique langfuse identifier of a trace | [required] |

### Return type

[**models::TraceWithFullDetails**](TraceWithFullDetails.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## trace_list

> models::Traces trace_list(page, limit, user_id, name, session_id, from_timestamp, to_timestamp, order_by, tags, version, release, environment, fields)


Get list of traces

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**i32**> | Page number, starts at 1 |  |
**limit** | Option<**i32**> | Limit of items per page. If you encounter api issues due to too large page sizes, try to reduce the limit. |  |
**user_id** | Option<**String**> |  |  |
**name** | Option<**String**> |  |  |
**session_id** | Option<**String**> |  |  |
**from_timestamp** | Option<**String**> | Optional filter to only include traces with a trace.timestamp on or after a certain datetime (ISO 8601) |  |
**to_timestamp** | Option<**String**> | Optional filter to only include traces with a trace.timestamp before a certain datetime (ISO 8601) |  |
**order_by** | Option<**String**> | Format of the string [field].[asc/desc]. Fields: id, timestamp, name, userId, release, version, public, bookmarked, sessionId. Example: timestamp.asc |  |
**tags** | Option<[**Vec<String>**](String.md)> | Only traces that include all of these tags will be returned. |  |
**version** | Option<**String**> | Optional filter to only include traces with a certain version. |  |
**release** | Option<**String**> | Optional filter to only include traces with a certain release. |  |
**environment** | Option<[**Vec<String>**](String.md)> | Optional filter for traces where the environment is one of the provided values. |  |
**fields** | Option<**String**> | Comma-separated list of fields to include in the response. Available field groups are 'core' (always included), 'io' (input, output, metadata), 'scores', 'observations', 'metrics'. If not provided, all fields are included. Example: 'core,scores,metrics' |  |

### Return type

[**models::Traces**](Traces.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

