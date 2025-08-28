# \ObservationsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**observations_get**](ObservationsApi.md#observations_get) | **GET** /api/public/observations/{observationId} | 
[**observations_get_many**](ObservationsApi.md#observations_get_many) | **GET** /api/public/observations | 



## observations_get

> models::ObservationsView observations_get(observation_id)


Get a observation

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**observation_id** | **String** | The unique langfuse identifier of an observation, can be an event, span or generation | [required] |

### Return type

[**models::ObservationsView**](ObservationsView.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## observations_get_many

> models::ObservationsViews observations_get_many(page, limit, name, user_id, r#type, trace_id, level, parent_observation_id, environment, from_start_time, to_start_time, version)


Get a list of observations

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**i32**> | Page number, starts at 1. |  |
**limit** | Option<**i32**> | Limit of items per page. If you encounter api issues due to too large page sizes, try to reduce the limit. |  |
**name** | Option<**String**> |  |  |
**user_id** | Option<**String**> |  |  |
**r#type** | Option<**String**> |  |  |
**trace_id** | Option<**String**> |  |  |
**level** | Option<[**ObservationLevel**](.md)> | Optional filter for observations with a specific level (e.g. \"DEBUG\", \"DEFAULT\", \"WARNING\", \"ERROR\"). |  |
**parent_observation_id** | Option<**String**> |  |  |
**environment** | Option<[**Vec<String>**](String.md)> | Optional filter for observations where the environment is one of the provided values. |  |
**from_start_time** | Option<**String**> | Retrieve only observations with a start_time on or after this datetime (ISO 8601). |  |
**to_start_time** | Option<**String**> | Retrieve only observations with a start_time before this datetime (ISO 8601). |  |
**version** | Option<**String**> | Optional filter to only include observations with a certain version. |  |

### Return type

[**models::ObservationsViews**](ObservationsViews.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

