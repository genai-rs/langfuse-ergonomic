# \MetricsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**metrics_metrics**](MetricsApi.md#metrics_metrics) | **GET** /api/public/metrics | 



## metrics_metrics

> models::MetricsResponse metrics_metrics(query)


Get metrics from the Langfuse project using a query object

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**query** | **String** | JSON string containing the query parameters with the following structure: ```json {   \"view\": string,           // Required. One of \"traces\", \"observations\", \"scores-numeric\", \"scores-categorical\"   \"dimensions\": [           // Optional. Default: []     {       \"field\": string       // Field to group by, e.g. \"name\", \"userId\", \"sessionId\"     }   ],   \"metrics\": [              // Required. At least one metric must be provided     {       \"measure\": string,    // What to measure, e.g. \"count\", \"latency\", \"value\"       \"aggregation\": string // How to aggregate, e.g. \"count\", \"sum\", \"avg\", \"p95\", \"histogram\"     }   ],   \"filters\": [              // Optional. Default: []     {       \"column\": string,     // Column to filter on       \"operator\": string,   // Operator, e.g. \"=\", \">\", \"<\", \"contains\"       \"value\": any,         // Value to compare against       \"type\": string,       // Data type, e.g. \"string\", \"number\", \"stringObject\"       \"key\": string         // Required only when filtering on metadata     }   ],   \"timeDimension\": {        // Optional. Default: null. If provided, results will be grouped by time     \"granularity\": string   // One of \"minute\", \"hour\", \"day\", \"week\", \"month\", \"auto\"   },   \"fromTimestamp\": string,  // Required. ISO datetime string for start of time range   \"toTimestamp\": string,    // Required. ISO datetime string for end of time range   \"orderBy\": [              // Optional. Default: null     {       \"field\": string,      // Field to order by       \"direction\": string   // \"asc\" or \"desc\"     }   ],   \"config\": {               // Optional. Query-specific configuration     \"bins\": number,         // Optional. Number of bins for histogram (1-100), default: 10     \"row_limit\": number     // Optional. Row limit for results (1-1000)   } } ``` | [required] |

### Return type

[**models::MetricsResponse**](MetricsResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

