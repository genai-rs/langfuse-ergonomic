# MetricsResponse

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**data** | [**Vec<std::collections::HashMap<String, serde_json::Value>>**](std::collections::HashMap.md) | The metrics data. Each item in the list contains the metric values and dimensions requested in the query. Format varies based on the query parameters. Histograms will return an array with [lower, upper, height] tuples. | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


