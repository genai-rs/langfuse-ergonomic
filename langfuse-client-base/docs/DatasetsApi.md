# \DatasetsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**datasets_create**](DatasetsApi.md#datasets_create) | **POST** /api/public/v2/datasets | 
[**datasets_delete_run**](DatasetsApi.md#datasets_delete_run) | **DELETE** /api/public/datasets/{datasetName}/runs/{runName} | 
[**datasets_get**](DatasetsApi.md#datasets_get) | **GET** /api/public/v2/datasets/{datasetName} | 
[**datasets_get_run**](DatasetsApi.md#datasets_get_run) | **GET** /api/public/datasets/{datasetName}/runs/{runName} | 
[**datasets_get_runs**](DatasetsApi.md#datasets_get_runs) | **GET** /api/public/datasets/{datasetName}/runs | 
[**datasets_list**](DatasetsApi.md#datasets_list) | **GET** /api/public/v2/datasets | 



## datasets_create

> models::Dataset datasets_create(create_dataset_request)


Create a dataset

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_dataset_request** | [**CreateDatasetRequest**](CreateDatasetRequest.md) |  | [required] |

### Return type

[**models::Dataset**](Dataset.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## datasets_delete_run

> models::DeleteDatasetRunResponse datasets_delete_run(dataset_name, run_name)


Delete a dataset run and all its run items. This action is irreversible.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**dataset_name** | **String** |  | [required] |
**run_name** | **String** |  | [required] |

### Return type

[**models::DeleteDatasetRunResponse**](DeleteDatasetRunResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## datasets_get

> models::Dataset datasets_get(dataset_name)


Get a dataset

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**dataset_name** | **String** |  | [required] |

### Return type

[**models::Dataset**](Dataset.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## datasets_get_run

> models::DatasetRunWithItems datasets_get_run(dataset_name, run_name)


Get a dataset run and its items

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**dataset_name** | **String** |  | [required] |
**run_name** | **String** |  | [required] |

### Return type

[**models::DatasetRunWithItems**](DatasetRunWithItems.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## datasets_get_runs

> models::PaginatedDatasetRuns datasets_get_runs(dataset_name, page, limit)


Get dataset runs

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**dataset_name** | **String** |  | [required] |
**page** | Option<**i32**> | page number, starts at 1 |  |
**limit** | Option<**i32**> | limit of items per page |  |

### Return type

[**models::PaginatedDatasetRuns**](PaginatedDatasetRuns.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## datasets_list

> models::PaginatedDatasets datasets_list(page, limit)


Get all datasets

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**i32**> | page number, starts at 1 |  |
**limit** | Option<**i32**> | limit of items per page |  |

### Return type

[**models::PaginatedDatasets**](PaginatedDatasets.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

