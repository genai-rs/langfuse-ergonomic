# \ModelsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**models_create**](ModelsApi.md#models_create) | **POST** /api/public/models | 
[**models_delete**](ModelsApi.md#models_delete) | **DELETE** /api/public/models/{id} | 
[**models_get**](ModelsApi.md#models_get) | **GET** /api/public/models/{id} | 
[**models_list**](ModelsApi.md#models_list) | **GET** /api/public/models | 



## models_create

> models::Model models_create(create_model_request)


Create a model

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_model_request** | [**CreateModelRequest**](CreateModelRequest.md) |  | [required] |

### Return type

[**models::Model**](Model.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## models_delete

> models_delete(id)


Delete a model. Cannot delete models managed by Langfuse. You can create your own definition with the same modelName to override the definition though.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** |  | [required] |

### Return type

 (empty response body)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## models_get

> models::Model models_get(id)


Get a model

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** |  | [required] |

### Return type

[**models::Model**](Model.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## models_list

> models::PaginatedModels models_list(page, limit)


Get all models

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**i32**> | page number, starts at 1 |  |
**limit** | Option<**i32**> | limit of items per page |  |

### Return type

[**models::PaginatedModels**](PaginatedModels.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

