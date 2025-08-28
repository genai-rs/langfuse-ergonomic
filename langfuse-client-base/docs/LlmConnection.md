# LlmConnection

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **String** |  | 
**provider** | **String** | Provider name (e.g., 'openai', 'my-gateway'). Must be unique in project, used for upserting. | 
**adapter** | **String** | The adapter used to interface with the LLM | 
**display_secret_key** | **String** | Masked version of the secret key for display purposes | 
**base_url** | Option<**String**> | Custom base URL for the LLM API | [optional]
**custom_models** | **Vec<String>** | List of custom model names available for this connection | 
**with_default_models** | **bool** | Whether to include default models for this adapter | 
**extra_header_keys** | **Vec<String>** | Keys of extra headers sent with requests (values excluded for security) | 
**created_at** | **String** |  | 
**updated_at** | **String** |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


