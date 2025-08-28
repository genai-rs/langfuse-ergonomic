# \SessionsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**sessions_get**](SessionsApi.md#sessions_get) | **GET** /api/public/sessions/{sessionId} | 
[**sessions_list**](SessionsApi.md#sessions_list) | **GET** /api/public/sessions | 



## sessions_get

> models::SessionWithTraces sessions_get(session_id)


Get a session. Please note that `traces` on this endpoint are not paginated, if you plan to fetch large sessions, consider `GET /api/public/traces?sessionId=<sessionId>`

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**session_id** | **String** | The unique id of a session | [required] |

### Return type

[**models::SessionWithTraces**](SessionWithTraces.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## sessions_list

> models::PaginatedSessions sessions_list(page, limit, from_timestamp, to_timestamp, environment)


Get sessions

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**i32**> | Page number, starts at 1 |  |
**limit** | Option<**i32**> | Limit of items per page. If you encounter api issues due to too large page sizes, try to reduce the limit. |  |
**from_timestamp** | Option<**String**> | Optional filter to only include sessions created on or after a certain datetime (ISO 8601) |  |
**to_timestamp** | Option<**String**> | Optional filter to only include sessions created before a certain datetime (ISO 8601) |  |
**environment** | Option<[**Vec<String>**](String.md)> | Optional filter for sessions where the environment is one of the provided values. |  |

### Return type

[**models::PaginatedSessions**](PaginatedSessions.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

