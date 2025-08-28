# CreateCommentRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**project_id** | **String** | The id of the project to attach the comment to. | 
**object_type** | **String** | The type of the object to attach the comment to (trace, observation, session, prompt). | 
**object_id** | **String** | The id of the object to attach the comment to. If this does not reference a valid existing object, an error will be thrown. | 
**content** | **String** | The content of the comment. May include markdown. Currently limited to 3000 characters. | 
**author_user_id** | Option<**String**> | The id of the user who created the comment. | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


