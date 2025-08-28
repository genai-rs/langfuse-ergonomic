# \LlmConnectionsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**llm_connections_list**](LlmConnectionsApi.md#llm_connections_list) | **GET** /api/public/llm-connections | 
[**llm_connections_upsert**](LlmConnectionsApi.md#llm_connections_upsert) | **PUT** /api/public/llm-connections | 



## llm_connections_list

> models::PaginatedLlmConnections llm_connections_list(page, limit)


Get all LLM connections in a project

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**i32**> | page number, starts at 1 |  |
**limit** | Option<**i32**> | limit of items per page |  |

### Return type

[**models::PaginatedLlmConnections**](PaginatedLlmConnections.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## llm_connections_upsert

> models::LlmConnection llm_connections_upsert(upsert_llm_connection_request)


Create or update an LLM connection. The connection is upserted on provider.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**upsert_llm_connection_request** | [**UpsertLlmConnectionRequest**](UpsertLlmConnectionRequest.md) |  | [required] |

### Return type

[**models::LlmConnection**](LlmConnection.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

