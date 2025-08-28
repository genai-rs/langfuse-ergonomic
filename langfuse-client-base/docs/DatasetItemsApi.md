# \DatasetItemsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**dataset_items_create**](DatasetItemsApi.md#dataset_items_create) | **POST** /api/public/dataset-items | 
[**dataset_items_delete**](DatasetItemsApi.md#dataset_items_delete) | **DELETE** /api/public/dataset-items/{id} | 
[**dataset_items_get**](DatasetItemsApi.md#dataset_items_get) | **GET** /api/public/dataset-items/{id} | 
[**dataset_items_list**](DatasetItemsApi.md#dataset_items_list) | **GET** /api/public/dataset-items | 



## dataset_items_create

> models::DatasetItem dataset_items_create(create_dataset_item_request)


Create a dataset item

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_dataset_item_request** | [**CreateDatasetItemRequest**](CreateDatasetItemRequest.md) |  | [required] |

### Return type

[**models::DatasetItem**](DatasetItem.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## dataset_items_delete

> models::DeleteDatasetItemResponse dataset_items_delete(id)


Delete a dataset item and all its run items. This action is irreversible.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** |  | [required] |

### Return type

[**models::DeleteDatasetItemResponse**](DeleteDatasetItemResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## dataset_items_get

> models::DatasetItem dataset_items_get(id)


Get a dataset item

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**id** | **String** |  | [required] |

### Return type

[**models::DatasetItem**](DatasetItem.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## dataset_items_list

> models::PaginatedDatasetItems dataset_items_list(dataset_name, source_trace_id, source_observation_id, page, limit)


Get dataset items

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**dataset_name** | Option<**String**> |  |  |
**source_trace_id** | Option<**String**> |  |  |
**source_observation_id** | Option<**String**> |  |  |
**page** | Option<**i32**> | page number, starts at 1 |  |
**limit** | Option<**i32**> | limit of items per page |  |

### Return type

[**models::PaginatedDatasetItems**](PaginatedDatasetItems.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

