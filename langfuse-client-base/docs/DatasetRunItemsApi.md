# \DatasetRunItemsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**dataset_run_items_create**](DatasetRunItemsApi.md#dataset_run_items_create) | **POST** /api/public/dataset-run-items | 
[**dataset_run_items_list**](DatasetRunItemsApi.md#dataset_run_items_list) | **GET** /api/public/dataset-run-items | 



## dataset_run_items_create

> models::DatasetRunItem dataset_run_items_create(create_dataset_run_item_request)


Create a dataset run item

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_dataset_run_item_request** | [**CreateDatasetRunItemRequest**](CreateDatasetRunItemRequest.md) |  | [required] |

### Return type

[**models::DatasetRunItem**](DatasetRunItem.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## dataset_run_items_list

> models::PaginatedDatasetRunItems dataset_run_items_list(dataset_id, run_name, page, limit)


List dataset run items

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**dataset_id** | **String** |  | [required] |
**run_name** | **String** |  | [required] |
**page** | Option<**i32**> | page number, starts at 1 |  |
**limit** | Option<**i32**> | limit of items per page |  |

### Return type

[**models::PaginatedDatasetRunItems**](PaginatedDatasetRunItems.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

