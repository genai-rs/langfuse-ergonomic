# \PromptVersionApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**prompt_version_update**](PromptVersionApi.md#prompt_version_update) | **PATCH** /api/public/v2/prompts/{name}/versions/{version} | 



## prompt_version_update

> models::Prompt prompt_version_update(name, version, prompt_version_update_request)


Update labels for a specific prompt version

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**name** | **String** | The name of the prompt | [required] |
**version** | **i32** | Version of the prompt to update | [required] |
**prompt_version_update_request** | [**PromptVersionUpdateRequest**](PromptVersionUpdateRequest.md) |  | [required] |

### Return type

[**models::Prompt**](Prompt.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

