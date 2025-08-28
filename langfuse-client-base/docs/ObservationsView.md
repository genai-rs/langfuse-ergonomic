# ObservationsView

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**prompt_name** | Option<**String**> | The name of the prompt associated with the observation | [optional]
**prompt_version** | Option<**i32**> | The version of the prompt associated with the observation | [optional]
**model_id** | Option<**String**> | The unique identifier of the model | [optional]
**input_price** | Option<**f64**> | The price of the input in USD | [optional]
**output_price** | Option<**f64**> | The price of the output in USD. | [optional]
**total_price** | Option<**f64**> | The total price in USD. | [optional]
**calculated_input_cost** | Option<**f64**> | (Deprecated. Use usageDetails and costDetails instead.) The calculated cost of the input in USD | [optional]
**calculated_output_cost** | Option<**f64**> | (Deprecated. Use usageDetails and costDetails instead.) The calculated cost of the output in USD | [optional]
**calculated_total_cost** | Option<**f64**> | (Deprecated. Use usageDetails and costDetails instead.) The calculated total cost in USD | [optional]
**latency** | Option<**f64**> | The latency in seconds. | [optional]
**time_to_first_token** | Option<**f64**> | The time to the first token in seconds | [optional]
**id** | **String** | The unique identifier of the observation | 
**trace_id** | Option<**String**> | The trace ID associated with the observation | [optional]
**r#type** | **String** | The type of the observation | 
**name** | Option<**String**> | The name of the observation | [optional]
**start_time** | **String** | The start time of the observation | 
**end_time** | Option<**String**> | The end time of the observation. | [optional]
**completion_start_time** | Option<**String**> | The completion start time of the observation | [optional]
**model** | Option<**String**> | The model used for the observation | [optional]
**model_parameters** | Option<[**std::collections::HashMap<String, models::MapValue>**](MapValue.md)> | The parameters of the model used for the observation | [optional]
**input** | Option<[**serde_json::Value**](.md)> | The input data of the observation | [optional]
**version** | Option<**String**> | The version of the observation | [optional]
**metadata** | Option<[**serde_json::Value**](.md)> | Additional metadata of the observation | [optional]
**output** | Option<[**serde_json::Value**](.md)> | The output data of the observation | [optional]
**usage** | Option<[**models::Usage**](Usage.md)> |  | [optional]
**level** | [**models::ObservationLevel**](ObservationLevel.md) |  | 
**status_message** | Option<**String**> | The status message of the observation | [optional]
**parent_observation_id** | Option<**String**> | The parent observation ID | [optional]
**prompt_id** | Option<**String**> | The prompt ID associated with the observation | [optional]
**usage_details** | Option<**std::collections::HashMap<String, i32>**> | The usage details of the observation. Key is the name of the usage metric, value is the number of units consumed. The total key is the sum of all (non-total) usage metrics or the total value ingested. | [optional]
**cost_details** | Option<**std::collections::HashMap<String, f64>**> | The cost details of the observation. Key is the name of the cost metric, value is the cost in USD. The total key is the sum of all (non-total) cost metrics or the total value ingested. | [optional]
**environment** | Option<**String**> | The environment from which this observation originated. Can be any lowercase alphanumeric string with hyphens and underscores that does not start with 'langfuse'. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


