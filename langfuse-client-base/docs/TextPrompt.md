# TextPrompt

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**prompt** | **String** |  | 
**name** | **String** |  | 
**version** | **i32** |  | 
**config** | Option<[**serde_json::Value**](.md)> |  | 
**labels** | **Vec<String>** | List of deployment labels of this prompt version. | 
**tags** | **Vec<String>** | List of tags. Used to filter via UI and API. The same across versions of a prompt. | 
**commit_message** | Option<**String**> | Commit message for this prompt version. | [optional]
**resolution_graph** | Option<[**std::collections::HashMap<String, serde_json::Value>**](serde_json::Value.md)> | The dependency resolution graph for the current prompt. Null if prompt has no dependencies. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


