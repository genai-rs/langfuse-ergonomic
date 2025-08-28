# \ProjectsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**projects_create**](ProjectsApi.md#projects_create) | **POST** /api/public/projects | 
[**projects_create_api_key**](ProjectsApi.md#projects_create_api_key) | **POST** /api/public/projects/{projectId}/apiKeys | 
[**projects_delete**](ProjectsApi.md#projects_delete) | **DELETE** /api/public/projects/{projectId} | 
[**projects_delete_api_key**](ProjectsApi.md#projects_delete_api_key) | **DELETE** /api/public/projects/{projectId}/apiKeys/{apiKeyId} | 
[**projects_get**](ProjectsApi.md#projects_get) | **GET** /api/public/projects | 
[**projects_get_api_keys**](ProjectsApi.md#projects_get_api_keys) | **GET** /api/public/projects/{projectId}/apiKeys | 
[**projects_update**](ProjectsApi.md#projects_update) | **PUT** /api/public/projects/{projectId} | 



## projects_create

> models::Project projects_create(projects_create_request)


Create a new project (requires organization-scoped API key)

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**projects_create_request** | [**ProjectsCreateRequest**](ProjectsCreateRequest.md) |  | [required] |

### Return type

[**models::Project**](Project.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## projects_create_api_key

> models::ApiKeyResponse projects_create_api_key(project_id, projects_create_api_key_request)


Create a new API key for a project (requires organization-scoped API key)

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project_id** | **String** |  | [required] |
**projects_create_api_key_request** | [**ProjectsCreateApiKeyRequest**](ProjectsCreateApiKeyRequest.md) |  | [required] |

### Return type

[**models::ApiKeyResponse**](ApiKeyResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## projects_delete

> models::ProjectDeletionResponse projects_delete(project_id)


Delete a project by ID (requires organization-scoped API key). Project deletion is processed asynchronously.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project_id** | **String** |  | [required] |

### Return type

[**models::ProjectDeletionResponse**](ProjectDeletionResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## projects_delete_api_key

> models::ApiKeyDeletionResponse projects_delete_api_key(project_id, api_key_id)


Delete an API key for a project (requires organization-scoped API key)

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project_id** | **String** |  | [required] |
**api_key_id** | **String** |  | [required] |

### Return type

[**models::ApiKeyDeletionResponse**](ApiKeyDeletionResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## projects_get

> models::Projects projects_get()


Get Project associated with API key

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::Projects**](Projects.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## projects_get_api_keys

> models::ApiKeyList projects_get_api_keys(project_id)


Get all API keys for a project (requires organization-scoped API key)

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project_id** | **String** |  | [required] |

### Return type

[**models::ApiKeyList**](ApiKeyList.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## projects_update

> models::Project projects_update(project_id, projects_create_request)


Update a project by ID (requires organization-scoped API key).

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project_id** | **String** |  | [required] |
**projects_create_request** | [**ProjectsCreateRequest**](ProjectsCreateRequest.md) |  | [required] |

### Return type

[**models::Project**](Project.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

