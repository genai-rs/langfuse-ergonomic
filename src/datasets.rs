//! Dataset management functionality
//!
//! This module contains types and utilities for dataset management.
//! The client methods live here as a `#[bon] impl` block on [`LangfuseClient`].

// Re-export common types that might be useful
pub use langfuse_client_base::models::CreateDatasetRequest;

use bon::bon;
use serde_json::Value;

use crate::client::LangfuseClient;
use crate::error::{Error, Result};

#[bon]
impl LangfuseClient {
    // ===== DATASET MANAGEMENT =====

    /// Create a dataset
    #[builder]
    pub async fn create_dataset(
        &self,
        #[builder(into)] name: String,
        #[builder(into)] description: Option<String>,
        metadata: Option<Value>,
        #[builder(into)] input_schema: Option<Value>,
        #[builder(into)] expected_output_schema: Option<Value>,
    ) -> Result<langfuse_client_base::models::Dataset> {
        use langfuse_client_base::apis::datasets_api;
        use langfuse_client_base::models::CreateDatasetRequest;

        let parse_schema = |value: Value| {
            if value.is_null() || value.is_object() {
                Ok(value)
            } else {
                // Keep diagnostics independent of caller-provided schema contents.
                Err(Error::Validation(
                    "Dataset schema must be a JSON object or null".to_string(),
                ))
            }
        };
        let input_schema = input_schema.map(parse_schema).transpose()?;
        let expected_output_schema = expected_output_schema.map(parse_schema).transpose()?;

        // The published client-base model uses a HashMap for these fields,
        // while generated source revisions use serde_json::Value. Deserialize
        // the wire representation so both supported model shapes remain usable.
        let mut request_body = serde_json::Map::new();
        request_body.insert("name".to_string(), Value::String(name));
        if let Some(description) = description {
            request_body.insert("description".to_string(), Value::String(description));
        }
        if let Some(metadata) = metadata {
            request_body.insert("metadata".to_string(), metadata);
        }
        if let Some(input_schema) = input_schema {
            request_body.insert("inputSchema".to_string(), input_schema);
        }
        if let Some(expected_output_schema) = expected_output_schema {
            request_body.insert("expectedOutputSchema".to_string(), expected_output_schema);
        }
        let request = serde_json::from_value::<CreateDatasetRequest>(Value::Object(request_body))
            .map_err(|_| {
            Error::Validation(
                "Dataset request is incompatible with the available API model".to_string(),
            )
        })?;

        datasets_api::datasets_create()
            .configuration(self.configuration())
            .create_dataset_request(request)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to create dataset: {}", e)))
    }

    /// Get a dataset by name
    pub async fn get_dataset(
        &self,
        dataset_name: impl Into<String>,
    ) -> Result<langfuse_client_base::models::Dataset> {
        use langfuse_client_base::apis::datasets_api;

        let dataset_name = dataset_name.into();

        datasets_api::datasets_get()
            .configuration(self.configuration())
            .dataset_name(dataset_name.as_str())
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get dataset: {}", e)))
    }

    /// List datasets with pagination
    #[builder]
    pub async fn list_datasets(
        &self,
        page: Option<i32>,
        limit: Option<i32>,
    ) -> Result<langfuse_client_base::models::PaginatedDatasets> {
        use langfuse_client_base::apis::datasets_api;

        datasets_api::datasets_list()
            .configuration(self.configuration())
            .maybe_page(page)
            .maybe_limit(limit)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to list datasets: {}", e)))
    }

    /// Delete a dataset run
    pub async fn delete_dataset_run(
        &self,
        dataset_name: impl Into<String>,
        run_name: impl Into<String>,
    ) -> Result<()> {
        use langfuse_client_base::apis::datasets_api;

        let dataset_name = dataset_name.into();
        let run_name = run_name.into();

        datasets_api::datasets_delete_run()
            .configuration(self.configuration())
            .dataset_name(dataset_name.as_str())
            .run_name(run_name.as_str())
            .call()
            .await
            .map(|_| ())
            .map_err(|e| crate::error::Error::Api(format!("Failed to delete dataset run: {}", e)))
    }

    /// Get a dataset run
    pub async fn get_dataset_run(
        &self,
        dataset_name: impl Into<String>,
        run_name: impl Into<String>,
    ) -> Result<langfuse_client_base::models::DatasetRunWithItems> {
        use langfuse_client_base::apis::datasets_api;

        let dataset_name = dataset_name.into();
        let run_name = run_name.into();

        datasets_api::datasets_get_run()
            .configuration(self.configuration())
            .dataset_name(dataset_name.as_str())
            .run_name(run_name.as_str())
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get dataset run: {}", e)))
    }

    /// Get all runs for a dataset
    pub async fn get_dataset_runs(
        &self,
        dataset_name: impl Into<String>,
    ) -> Result<langfuse_client_base::models::PaginatedDatasetRuns> {
        use langfuse_client_base::apis::datasets_api;

        let dataset_name = dataset_name.into();

        datasets_api::datasets_get_runs()
            .configuration(self.configuration())
            .dataset_name(dataset_name.as_str())
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get dataset runs: {}", e)))
    }

    // ===== DATASET ITEM OPERATIONS =====

    /// Create a new dataset item
    #[builder]
    pub async fn create_dataset_item(
        &self,
        #[builder(into)] dataset_name: String,
        input: Option<Value>,
        expected_output: Option<Value>,
        metadata: Option<Value>,
        #[builder(into)] source_trace_id: Option<String>,
        #[builder(into)] source_observation_id: Option<String>,
        #[builder(into)] id: Option<String>,
        _status: Option<String>,
    ) -> Result<langfuse_client_base::models::DatasetItem> {
        use langfuse_client_base::apis::dataset_items_api;
        use langfuse_client_base::models::CreateDatasetItemRequest;

        let item_request = CreateDatasetItemRequest {
            dataset_name,
            input: Some(input),
            expected_output: Some(expected_output),
            metadata: Some(metadata),
            source_trace_id: Some(source_trace_id),
            source_observation_id: Some(source_observation_id),
            id: Some(id),
            status: None, // Status field requires DatasetStatus enum, not available in public API
        };

        dataset_items_api::dataset_items_create()
            .configuration(self.configuration())
            .create_dataset_item_request(item_request)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to create dataset item: {}", e)))
    }

    /// Get a specific dataset item
    pub async fn get_dataset_item(
        &self,
        item_id: impl Into<String>,
    ) -> Result<langfuse_client_base::models::DatasetItem> {
        use langfuse_client_base::apis::dataset_items_api;

        let item_id = item_id.into();

        dataset_items_api::dataset_items_get()
            .configuration(self.configuration())
            .id(item_id.as_str())
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get dataset item: {}", e)))
    }

    /// List dataset items
    #[builder]
    pub async fn list_dataset_items(
        &self,
        #[builder(into)] dataset_name: Option<String>,
        #[builder(into)] source_trace_id: Option<String>,
        #[builder(into)] source_observation_id: Option<String>,
        page: Option<i32>,
        limit: Option<i32>,
    ) -> Result<langfuse_client_base::models::PaginatedDatasetItems> {
        use langfuse_client_base::apis::dataset_items_api;

        let dataset_name_ref = dataset_name.as_deref();
        let source_trace_ref = source_trace_id.as_deref();
        let source_observation_ref = source_observation_id.as_deref();

        dataset_items_api::dataset_items_list()
            .configuration(self.configuration())
            .maybe_dataset_name(dataset_name_ref)
            .maybe_source_trace_id(source_trace_ref)
            .maybe_source_observation_id(source_observation_ref)
            .maybe_page(page)
            .maybe_limit(limit)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to list dataset items: {}", e)))
    }

    /// Delete a dataset item
    pub async fn delete_dataset_item(&self, item_id: impl Into<String>) -> Result<()> {
        use langfuse_client_base::apis::dataset_items_api;

        let item_id = item_id.into();

        dataset_items_api::dataset_items_delete()
            .configuration(self.configuration())
            .id(item_id.as_str())
            .call()
            .await
            .map_err(|e| {
                crate::error::Error::Api(format!("Failed to delete dataset item: {}", e))
            })?;

        Ok(())
    }

    // Note: dataset_run_items_api doesn't exist in v0.2
    // We'll implement this when the API is available
}
