# CreateModelRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**model_name** | **String** | Name of the model definition. If multiple with the same name exist, they are applied in the following order: (1) custom over built-in, (2) newest according to startTime where model.startTime<observation.startTime | 
**match_pattern** | **String** | Regex pattern which matches this model definition to generation.model. Useful in case of fine-tuned models. If you want to exact match, use `(?i)^modelname$` | 
**start_date** | Option<**String**> | Apply only to generations which are newer than this ISO date. | [optional]
**unit** | Option<[**models::ModelUsageUnit**](ModelUsageUnit.md)> |  | [optional]
**input_price** | Option<**f64**> | Price (USD) per input unit | [optional]
**output_price** | Option<**f64**> | Price (USD) per output unit | [optional]
**total_price** | Option<**f64**> | Price (USD) per total units. Cannot be set if input or output price is set. | [optional]
**tokenizer_id** | Option<**String**> | Optional. Tokenizer to be applied to observations which match to this model. See docs for more details. | [optional]
**tokenizer_config** | Option<[**serde_json::Value**](.md)> | Optional. Configuration for the selected tokenizer. Needs to be JSON. See docs for more details. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


