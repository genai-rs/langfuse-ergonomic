# UpsertLlmConnectionRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**provider** | **String** | Provider name (e.g., 'openai', 'my-gateway'). Must be unique in project, used for upserting. | 
**adapter** | [**models::LlmAdapter**](LlmAdapter.md) |  | 
**secret_key** | **String** | Secret key for the LLM API. | 
**base_url** | Option<**String**> | Custom base URL for the LLM API | [optional]
**custom_models** | Option<**Vec<String>**> | List of custom model names | [optional]
**with_default_models** | Option<**bool**> | Whether to include default models. Default is true. | [optional]
**extra_headers** | Option<**std::collections::HashMap<String, String>**> | Extra headers to send with requests | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


