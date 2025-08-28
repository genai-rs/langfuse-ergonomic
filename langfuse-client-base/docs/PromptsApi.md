# \PromptsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**prompts_create**](PromptsApi.md#prompts_create) | **POST** /api/public/v2/prompts | 
[**prompts_get**](PromptsApi.md#prompts_get) | **GET** /api/public/v2/prompts/{promptName} | 
[**prompts_list**](PromptsApi.md#prompts_list) | **GET** /api/public/v2/prompts | 



## prompts_create

> models::Prompt prompts_create(create_prompt_request)


Create a new version for the prompt with the given `name`

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_prompt_request** | [**CreatePromptRequest**](CreatePromptRequest.md) |  | [required] |

### Return type

[**models::Prompt**](Prompt.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## prompts_get

> models::Prompt prompts_get(prompt_name, version, label)


Get a prompt

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**prompt_name** | **String** | The name of the prompt | [required] |
**version** | Option<**i32**> | Version of the prompt to be retrieved. |  |
**label** | Option<**String**> | Label of the prompt to be retrieved. Defaults to \"production\" if no label or version is set. |  |

### Return type

[**models::Prompt**](Prompt.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## prompts_list

> models::PromptMetaListResponse prompts_list(name, label, tag, page, limit, from_updated_at, to_updated_at)


Get a list of prompt names with versions and labels

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**name** | Option<**String**> |  |  |
**label** | Option<**String**> |  |  |
**tag** | Option<**String**> |  |  |
**page** | Option<**i32**> | page number, starts at 1 |  |
**limit** | Option<**i32**> | limit of items per page |  |
**from_updated_at** | Option<**String**> | Optional filter to only include prompt versions created/updated on or after a certain datetime (ISO 8601) |  |
**to_updated_at** | Option<**String**> | Optional filter to only include prompt versions created/updated before a certain datetime (ISO 8601) |  |

### Return type

[**models::PromptMetaListResponse**](PromptMetaListResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

