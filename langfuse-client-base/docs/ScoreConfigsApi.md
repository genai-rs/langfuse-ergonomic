# \ScoreConfigsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**score_configs_create**](ScoreConfigsApi.md#score_configs_create) | **POST** /api/public/score-configs | 
[**score_configs_get**](ScoreConfigsApi.md#score_configs_get) | **GET** /api/public/score-configs | 
[**score_configs_get_by_id**](ScoreConfigsApi.md#score_configs_get_by_id) | **GET** /api/public/score-configs/{configId} | 



## score_configs_create

> models::ScoreConfig score_configs_create(create_score_config_request)


Create a score configuration (config). Score configs are used to define the structure of scores

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_score_config_request** | [**CreateScoreConfigRequest**](CreateScoreConfigRequest.md) |  | [required] |

### Return type

[**models::ScoreConfig**](ScoreConfig.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## score_configs_get

> models::ScoreConfigs score_configs_get(page, limit)


Get all score configs

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**i32**> | Page number, starts at 1. |  |
**limit** | Option<**i32**> | Limit of items per page. If you encounter api issues due to too large page sizes, try to reduce the limit |  |

### Return type

[**models::ScoreConfigs**](ScoreConfigs.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## score_configs_get_by_id

> models::ScoreConfig score_configs_get_by_id(config_id)


Get a score config

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**config_id** | **String** | The unique langfuse identifier of a score config | [required] |

### Return type

[**models::ScoreConfig**](ScoreConfig.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

