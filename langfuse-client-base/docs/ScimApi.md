# \ScimApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**scim_create_user**](ScimApi.md#scim_create_user) | **POST** /api/public/scim/Users | 
[**scim_delete_user**](ScimApi.md#scim_delete_user) | **DELETE** /api/public/scim/Users/{userId} | 
[**scim_get_resource_types**](ScimApi.md#scim_get_resource_types) | **GET** /api/public/scim/ResourceTypes | 
[**scim_get_schemas**](ScimApi.md#scim_get_schemas) | **GET** /api/public/scim/Schemas | 
[**scim_get_service_provider_config**](ScimApi.md#scim_get_service_provider_config) | **GET** /api/public/scim/ServiceProviderConfig | 
[**scim_get_user**](ScimApi.md#scim_get_user) | **GET** /api/public/scim/Users/{userId} | 
[**scim_list_users**](ScimApi.md#scim_list_users) | **GET** /api/public/scim/Users | 



## scim_create_user

> models::ScimUser scim_create_user(scim_create_user_request)


Create a new user in the organization (requires organization-scoped API key)

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**scim_create_user_request** | [**ScimCreateUserRequest**](ScimCreateUserRequest.md) |  | [required] |

### Return type

[**models::ScimUser**](ScimUser.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## scim_delete_user

> serde_json::Value scim_delete_user(user_id)


Remove a user from the organization (requires organization-scoped API key). Note that this only removes the user from the organization but does not delete the user entity itself.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** |  | [required] |

### Return type

[**serde_json::Value**](serde_json::Value.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## scim_get_resource_types

> models::ResourceTypesResponse scim_get_resource_types()


Get SCIM Resource Types (requires organization-scoped API key)

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::ResourceTypesResponse**](ResourceTypesResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## scim_get_schemas

> models::SchemasResponse scim_get_schemas()


Get SCIM Schemas (requires organization-scoped API key)

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::SchemasResponse**](SchemasResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## scim_get_service_provider_config

> models::ServiceProviderConfig scim_get_service_provider_config()


Get SCIM Service Provider Configuration (requires organization-scoped API key)

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::ServiceProviderConfig**](ServiceProviderConfig.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## scim_get_user

> models::ScimUser scim_get_user(user_id)


Get a specific user by ID (requires organization-scoped API key)

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **String** |  | [required] |

### Return type

[**models::ScimUser**](ScimUser.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## scim_list_users

> models::ScimUsersListResponse scim_list_users(filter, start_index, count)


List users in the organization (requires organization-scoped API key)

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**filter** | Option<**String**> | Filter expression (e.g. userName eq \"value\") |  |
**start_index** | Option<**i32**> | 1-based index of the first result to return (default 1) |  |
**count** | Option<**i32**> | Maximum number of results to return (default 100) |  |

### Return type

[**models::ScimUsersListResponse**](ScimUsersListResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

