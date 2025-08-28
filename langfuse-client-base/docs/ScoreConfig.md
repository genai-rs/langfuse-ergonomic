# ScoreConfig

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **String** |  | 
**name** | **String** |  | 
**created_at** | **String** |  | 
**updated_at** | **String** |  | 
**project_id** | **String** |  | 
**data_type** | [**models::ScoreDataType**](ScoreDataType.md) |  | 
**is_archived** | **bool** | Whether the score config is archived. Defaults to false | 
**min_value** | Option<**f64**> | Sets minimum value for numerical scores. If not set, the minimum value defaults to -∞ | [optional]
**max_value** | Option<**f64**> | Sets maximum value for numerical scores. If not set, the maximum value defaults to +∞ | [optional]
**categories** | Option<[**Vec<models::ConfigCategory>**](ConfigCategory.md)> | Configures custom categories for categorical scores | [optional]
**description** | Option<**String**> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


