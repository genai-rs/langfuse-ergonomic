# \OrganizationsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**organizations_delete_organization_membership**](OrganizationsApi.md#organizations_delete_organization_membership) | **DELETE** /api/public/organizations/memberships | 
[**organizations_delete_project_membership**](OrganizationsApi.md#organizations_delete_project_membership) | **DELETE** /api/public/projects/{projectId}/memberships | 
[**organizations_get_organization_memberships**](OrganizationsApi.md#organizations_get_organization_memberships) | **GET** /api/public/organizations/memberships | 
[**organizations_get_organization_projects**](OrganizationsApi.md#organizations_get_organization_projects) | **GET** /api/public/organizations/projects | 
[**organizations_get_project_memberships**](OrganizationsApi.md#organizations_get_project_memberships) | **GET** /api/public/projects/{projectId}/memberships | 
[**organizations_update_organization_membership**](OrganizationsApi.md#organizations_update_organization_membership) | **PUT** /api/public/organizations/memberships | 
[**organizations_update_project_membership**](OrganizationsApi.md#organizations_update_project_membership) | **PUT** /api/public/projects/{projectId}/memberships | 



## organizations_delete_organization_membership

> models::MembershipDeletionResponse organizations_delete_organization_membership(delete_membership_request)


Delete a membership from the organization associated with the API key (requires organization-scoped API key)

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**delete_membership_request** | [**DeleteMembershipRequest**](DeleteMembershipRequest.md) |  | [required] |

### Return type

[**models::MembershipDeletionResponse**](MembershipDeletionResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## organizations_delete_project_membership

> models::MembershipDeletionResponse organizations_delete_project_membership(project_id, delete_membership_request)


Delete a membership from a specific project (requires organization-scoped API key). The user must be a member of the organization.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project_id** | **String** |  | [required] |
**delete_membership_request** | [**DeleteMembershipRequest**](DeleteMembershipRequest.md) |  | [required] |

### Return type

[**models::MembershipDeletionResponse**](MembershipDeletionResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## organizations_get_organization_memberships

> models::MembershipsResponse organizations_get_organization_memberships()


Get all memberships for the organization associated with the API key (requires organization-scoped API key)

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::MembershipsResponse**](MembershipsResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## organizations_get_organization_projects

> models::OrganizationProjectsResponse organizations_get_organization_projects()


Get all projects for the organization associated with the API key (requires organization-scoped API key)

### Parameters

This endpoint does not need any parameter.

### Return type

[**models::OrganizationProjectsResponse**](OrganizationProjectsResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## organizations_get_project_memberships

> models::MembershipsResponse organizations_get_project_memberships(project_id)


Get all memberships for a specific project (requires organization-scoped API key)

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project_id** | **String** |  | [required] |

### Return type

[**models::MembershipsResponse**](MembershipsResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## organizations_update_organization_membership

> models::MembershipResponse organizations_update_organization_membership(membership_request)


Create or update a membership for the organization associated with the API key (requires organization-scoped API key)

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**membership_request** | [**MembershipRequest**](MembershipRequest.md) |  | [required] |

### Return type

[**models::MembershipResponse**](MembershipResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## organizations_update_project_membership

> models::MembershipResponse organizations_update_project_membership(project_id, membership_request)


Create or update a membership for a specific project (requires organization-scoped API key). The user must already be a member of the organization.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**project_id** | **String** |  | [required] |
**membership_request** | [**MembershipRequest**](MembershipRequest.md) |  | [required] |

### Return type

[**models::MembershipResponse**](MembershipResponse.md)

### Authorization

[BasicAuth](../README.md#BasicAuth)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

