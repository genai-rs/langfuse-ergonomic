# ScimCreateUserRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**user_name** | **String** | User's email address (required) | 
**name** | [**models::ScimName**](ScimName.md) |  | 
**emails** | Option<[**Vec<models::ScimEmail>**](ScimEmail.md)> | User's email addresses | [optional]
**active** | Option<**bool**> | Whether the user is active | [optional]
**password** | Option<**String**> | Initial password for the user | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


