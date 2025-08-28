# \AnnotationQueuesApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**annotation_queues_create_queue**](AnnotationQueuesApi.md#annotation_queues_create_queue) | **POST** /api/public/annotation-queues | 
[**annotation_queues_create_queue_assignment**](AnnotationQueuesApi.md#annotation_queues_create_queue_assignment) | **POST** /api/public/annotation-queues/{queueId}/assignments | 
[**annotation_queues_create_queue_item**](AnnotationQueuesApi.md#annotation_queues_create_queue_item) | **POST** /api/public/annotation-queues/{queueId}/items | 
[**annotation_queues_delete_queue_assignment**](AnnotationQueuesApi.md#annotation_queues_delete_queue_assignment) | **DELETE** /api/public/annotation-queues/{queueId}/assignments | 
[**annotation_queues_delete_queue_item**](AnnotationQueuesApi.md#annotation_queues_delete_queue_item) | **DELETE** /api/public/annotation-queues/{queueId}/items/{itemId} | 
[**annotation_queues_get_queue**](AnnotationQueuesApi.md#annotation_queues_get_queue) | **GET** /api/public/annotation-queues/{queueId} | 
[**annotation_queues_get_queue_item**](AnnotationQueuesApi.md#annotation_queues_get_queue_item) | **GET** /api/public/annotation-queues/{queueId}/items/{itemId} | 
[**annotation_queues_list_queue_items**](AnnotationQueuesApi.md#annotation_queues_list_queue_items) | **GET** /api/public/annotation-queues/{queueId}/items | 
[**annotation_queues_list_queues**](AnnotationQueuesApi.md#annotation_queues_list_queues) | **GET** /api/public/annotation-queues | 
[**annotation_queues_update_queue_item**](AnnotationQueuesApi.md#annotation_queues_update_queue_item) | **PATCH** /api/public/annotation-queues/{queueId}/items/{itemId} | 



## annotation_queues_create_queue

> models::AnnotationQueue annotation_queues_create_queue(create_annotation_queue_request)


Create an annotation queue

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**create_annotation_queue_request** | [**CreateAnnotationQueueRequest**](CreateAnnotationQueueRequest.md) |  | [required] |

### Return type

[**models::AnnotationQueue**](AnnotationQueue.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## annotation_queues_create_queue_assignment

> models::CreateAnnotationQueueAssignmentResponse annotation_queues_create_queue_assignment(queue_id, annotation_queue_assignment_request)


Create an assignment for a user to an annotation queue

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**queue_id** | **String** | The unique identifier of the annotation queue | [required] |
**annotation_queue_assignment_request** | [**AnnotationQueueAssignmentRequest**](AnnotationQueueAssignmentRequest.md) |  | [required] |

### Return type

[**models::CreateAnnotationQueueAssignmentResponse**](CreateAnnotationQueueAssignmentResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## annotation_queues_create_queue_item

> models::AnnotationQueueItem annotation_queues_create_queue_item(queue_id, create_annotation_queue_item_request)


Add an item to an annotation queue

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**queue_id** | **String** | The unique identifier of the annotation queue | [required] |
**create_annotation_queue_item_request** | [**CreateAnnotationQueueItemRequest**](CreateAnnotationQueueItemRequest.md) |  | [required] |

### Return type

[**models::AnnotationQueueItem**](AnnotationQueueItem.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## annotation_queues_delete_queue_assignment

> models::DeleteAnnotationQueueAssignmentResponse annotation_queues_delete_queue_assignment(queue_id, annotation_queue_assignment_request)


Delete an assignment for a user to an annotation queue

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**queue_id** | **String** | The unique identifier of the annotation queue | [required] |
**annotation_queue_assignment_request** | [**AnnotationQueueAssignmentRequest**](AnnotationQueueAssignmentRequest.md) |  | [required] |

### Return type

[**models::DeleteAnnotationQueueAssignmentResponse**](DeleteAnnotationQueueAssignmentResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## annotation_queues_delete_queue_item

> models::DeleteAnnotationQueueItemResponse annotation_queues_delete_queue_item(queue_id, item_id)


Remove an item from an annotation queue

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**queue_id** | **String** | The unique identifier of the annotation queue | [required] |
**item_id** | **String** | The unique identifier of the annotation queue item | [required] |

### Return type

[**models::DeleteAnnotationQueueItemResponse**](DeleteAnnotationQueueItemResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## annotation_queues_get_queue

> models::AnnotationQueue annotation_queues_get_queue(queue_id)


Get an annotation queue by ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**queue_id** | **String** | The unique identifier of the annotation queue | [required] |

### Return type

[**models::AnnotationQueue**](AnnotationQueue.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## annotation_queues_get_queue_item

> models::AnnotationQueueItem annotation_queues_get_queue_item(queue_id, item_id)


Get a specific item from an annotation queue

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**queue_id** | **String** | The unique identifier of the annotation queue | [required] |
**item_id** | **String** | The unique identifier of the annotation queue item | [required] |

### Return type

[**models::AnnotationQueueItem**](AnnotationQueueItem.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## annotation_queues_list_queue_items

> models::PaginatedAnnotationQueueItems annotation_queues_list_queue_items(queue_id, status, page, limit)


Get items for a specific annotation queue

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**queue_id** | **String** | The unique identifier of the annotation queue | [required] |
**status** | Option<[**AnnotationQueueStatus**](.md)> | Filter by status |  |
**page** | Option<**i32**> | page number, starts at 1 |  |
**limit** | Option<**i32**> | limit of items per page |  |

### Return type

[**models::PaginatedAnnotationQueueItems**](PaginatedAnnotationQueueItems.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## annotation_queues_list_queues

> models::PaginatedAnnotationQueues annotation_queues_list_queues(page, limit)


Get all annotation queues

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**page** | Option<**i32**> | page number, starts at 1 |  |
**limit** | Option<**i32**> | limit of items per page |  |

### Return type

[**models::PaginatedAnnotationQueues**](PaginatedAnnotationQueues.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## annotation_queues_update_queue_item

> models::AnnotationQueueItem annotation_queues_update_queue_item(queue_id, item_id, update_annotation_queue_item_request)


Update an annotation queue item

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**queue_id** | **String** | The unique identifier of the annotation queue | [required] |
**item_id** | **String** | The unique identifier of the annotation queue item | [required] |
**update_annotation_queue_item_request** | [**UpdateAnnotationQueueItemRequest**](UpdateAnnotationQueueItemRequest.md) |  | [required] |

### Return type

[**models::AnnotationQueueItem**](AnnotationQueueItem.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

