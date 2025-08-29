//! Batch ingestion with automatic chunking, retries, and 207 handling

use bon::bon;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::time::interval;

use crate::client::LangfuseClient;
use crate::error::{Error, EventError, IngestionResponse, Result};
use langfuse_client_base::models::{IngestionBatchRequest, IngestionEvent};

/// Maximum batch size in bytes (3.5 MB as per Langfuse docs)
const MAX_BATCH_SIZE_BYTES: usize = 3_500_000;

/// Default maximum events per batch
const DEFAULT_MAX_EVENTS: usize = 100;

/// Default flush interval
const DEFAULT_FLUSH_INTERVAL: Duration = Duration::from_secs(5);

/// Default retry attempts
const DEFAULT_MAX_RETRIES: u32 = 3;

/// Event wrapper with metadata for batching
#[derive(Debug, Clone)]
pub struct BatchEvent {
    /// The actual ingestion event
    pub event: IngestionEvent,
    /// Unique ID for tracking
    pub id: String,
    /// Size in bytes (serialized)
    pub size: usize,
    /// Number of retry attempts
    pub retry_count: u32,
}

impl BatchEvent {
    /// Create a new batch event
    pub fn new(event: IngestionEvent, id: String) -> Result<Self> {
        let serialized = serde_json::to_vec(&event)?;
        Ok(Self {
            event,
            id,
            size: serialized.len(),
            retry_count: 0,
        })
    }
}

/// Configuration for the batcher
#[derive(Debug, Clone)]
pub struct BatcherConfig {
    /// Maximum number of events per batch
    pub max_events: usize,
    /// Maximum batch size in bytes
    pub max_bytes: usize,
    /// How often to flush the batch
    pub flush_interval: Duration,
    /// Maximum retry attempts for failed events
    pub max_retries: u32,
    /// Initial retry delay
    pub initial_retry_delay: Duration,
    /// Maximum retry delay
    pub max_retry_delay: Duration,
    /// Whether to fail fast on errors or continue with partial failures
    pub fail_fast: bool,
}

impl Default for BatcherConfig {
    fn default() -> Self {
        Self {
            max_events: DEFAULT_MAX_EVENTS,
            max_bytes: MAX_BATCH_SIZE_BYTES,
            flush_interval: DEFAULT_FLUSH_INTERVAL,
            max_retries: DEFAULT_MAX_RETRIES,
            initial_retry_delay: Duration::from_millis(100),
            max_retry_delay: Duration::from_secs(30),
            fail_fast: false,
        }
    }
}

/// Batch ingestion handler with automatic chunking and retries
pub struct Batcher {
    client: Arc<LangfuseClient>,
    config: BatcherConfig,
    buffer: Arc<Mutex<Vec<BatchEvent>>>,
    tx: mpsc::Sender<BatchEvent>,
    rx: Arc<Mutex<mpsc::Receiver<BatchEvent>>>,
    shutdown_tx: mpsc::Sender<()>,
}

#[bon]
impl Batcher {
    /// Create a new batcher with custom configuration
    #[builder]
    pub fn new(
        client: LangfuseClient,
        max_events: Option<usize>,
        max_bytes: Option<usize>,
        flush_interval: Option<Duration>,
        max_retries: Option<u32>,
        fail_fast: Option<bool>,
    ) -> Self {
        let config = BatcherConfig {
            max_events: max_events.unwrap_or(DEFAULT_MAX_EVENTS),
            max_bytes: max_bytes.unwrap_or(MAX_BATCH_SIZE_BYTES),
            flush_interval: flush_interval.unwrap_or(DEFAULT_FLUSH_INTERVAL),
            max_retries: max_retries.unwrap_or(DEFAULT_MAX_RETRIES),
            fail_fast: fail_fast.unwrap_or(false),
            ..Default::default()
        };

        let (tx, rx) = mpsc::channel(1000);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

        let batcher = Self {
            client: Arc::new(client),
            config: config.clone(),
            buffer: Arc::new(Mutex::new(Vec::new())),
            tx,
            rx: Arc::new(Mutex::new(rx)),
            shutdown_tx,
        };

        // Start background flush task
        let buffer = batcher.buffer.clone();
        let client = batcher.client.clone();
        let rx = batcher.rx.clone();

        tokio::spawn(async move {
            let mut flush_interval = interval(config.flush_interval);
            flush_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            // Consume the first immediate tick
            flush_interval.tick().await;

            loop {
                tokio::select! {
                    _ = flush_interval.tick() => {
                        let _ = Self::flush_buffer(&client, &buffer, &config).await;
                    }
                    Some(event) = async {
                        let mut rx = rx.lock().await;
                        rx.recv().await
                    } => {
                        let should_flush = {
                            let mut buf = buffer.lock().await;
                            buf.push(event);

                            // Check if we should flush based on size/count
                            let total_size: usize = buf.iter().map(|e| e.size).sum();
                            let len = buf.len();
                            len >= config.max_events || total_size >= config.max_bytes
                        };

                        if should_flush {
                            let _ = Self::flush_buffer(&client, &buffer, &config).await;
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        // Final flush before shutdown
                        let _ = Self::flush_buffer(&client, &buffer, &config).await;
                        break;
                    }
                }
            }
        });

        batcher
    }

    /// Add an event to the batch
    pub async fn add(&self, event: IngestionEvent) -> Result<()> {
        let id = match &event {
            IngestionEvent::IngestionEventOneOf(e) => e.id.clone(),
            IngestionEvent::IngestionEventOneOf1(e) => e.id.clone(),
            IngestionEvent::IngestionEventOneOf2(e) => e.id.clone(),
            IngestionEvent::IngestionEventOneOf3(e) => e.id.clone(),
            IngestionEvent::IngestionEventOneOf4(e) => e.id.clone(),
            IngestionEvent::IngestionEventOneOf5(e) => e.id.clone(),
            IngestionEvent::IngestionEventOneOf6(e) => e.id.clone(),
            _ => uuid::Uuid::new_v4().to_string(),
        };
        

        let batch_event = BatchEvent::new(event, id.clone())?;

        // Check size limit
        if batch_event.size > self.config.max_bytes {
            return Err(Error::BatchSizeExceeded {
                size: batch_event.size,
                max_size: self.config.max_bytes,
            });
        }

        self.tx
            .send(batch_event)
            .await
            .map_err(|e| Error::Api(format!("Failed to queue event: {}", e)))?;
            

        Ok(())
    }

    /// Manually flush the current batch
    pub async fn flush(&self) -> Result<IngestionResponse> {
        // Give background task time to add pending events to buffer
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // Flush the buffer
        Self::flush_buffer(&self.client, &self.buffer, &self.config).await
    }

    /// Internal flush implementation
    async fn flush_buffer(
        client: &LangfuseClient,
        buffer: &Arc<Mutex<Vec<BatchEvent>>>,
        config: &BatcherConfig,
    ) -> Result<IngestionResponse> {
        let mut events = {
            let mut buffer = buffer.lock().await;
            std::mem::take(&mut *buffer)
        };
        

        if events.is_empty() {
            return Ok(IngestionResponse {
                success_ids: vec![],
                failures: vec![],
                success_count: 0,
                failure_count: 0,
            });
        }

        // Split into chunks that fit size limit
        let chunks = Self::chunk_events(&events, config.max_bytes, config.max_events);

        let mut all_success_ids = Vec::new();
        let mut all_failures = Vec::new();
        let mut retry_queue = Vec::new();

        for chunk in chunks {
            match Self::send_batch_with_retry(client, &chunk, config).await {
                Ok(response) => {
                    all_success_ids.extend(response.success_ids.clone());

                    // Queue retryable failures
                    for failure in &response.failures {
                        if failure.retryable {
                            if let Some(event) =
                                events.iter_mut().find(|e| e.id == failure.event_id)
                            {
                                if event.retry_count < config.max_retries {
                                    event.retry_count += 1;
                                    retry_queue.push(event.clone());
                                }
                            }
                        }
                    }
                    all_failures.extend(response.failures);
                }
                Err(e) if e.is_retryable() => {
                    // Queue all events for retry
                    for event in &chunk {
                        if event.retry_count < config.max_retries {
                            let mut event = event.clone();
                            event.retry_count += 1;
                            retry_queue.push(event);
                        }
                    }
                }
                Err(e) => {
                    // Always fail fast for auth errors
                    if matches!(e, Error::Auth { .. }) || config.fail_fast {
                        return Err(e);
                    }
                    // Convert to failures
                    for event in &chunk {
                        all_failures.push(EventError {
                            event_id: event.id.clone(),
                            message: e.to_string(),
                            code: None,
                            retryable: false,
                        });
                    }
                }
            }
        }

        // Re-queue retry events
        if !retry_queue.is_empty() {
            let mut buffer = buffer.lock().await;
            buffer.extend(retry_queue);
        }

        Ok(IngestionResponse {
            success_ids: all_success_ids.clone(),
            failures: all_failures.clone(),
            success_count: all_success_ids.len(),
            failure_count: all_failures.len(),
        })
    }

    /// Split events into chunks that fit size and count limits
    fn chunk_events(
        events: &[BatchEvent],
        max_bytes: usize,
        max_events: usize,
    ) -> Vec<Vec<BatchEvent>> {
        let mut chunks = Vec::new();
        let mut current_chunk = Vec::new();
        let mut current_size = 0;

        for event in events {
            if current_chunk.len() >= max_events
                || (current_size + event.size > max_bytes && !current_chunk.is_empty())
            {
                chunks.push(current_chunk);
                current_chunk = Vec::new();
                current_size = 0;
            }

            current_size += event.size;
            current_chunk.push(event.clone());
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }

    /// Send a batch with exponential backoff retry
    async fn send_batch_with_retry(
        client: &LangfuseClient,
        events: &[BatchEvent],
        config: &BatcherConfig,
    ) -> Result<IngestionResponse> {
        let mut delay = config.initial_retry_delay;
        let mut last_error = None;

        for attempt in 0..=config.max_retries {
            if attempt > 0 {
                tokio::time::sleep(delay).await;
                delay = std::cmp::min(delay * 2, config.max_retry_delay);
            }

            let batch_request = IngestionBatchRequest {
                batch: events.iter().map(|e| e.event.clone()).collect(),
                metadata: None,
            };

            match Self::send_batch_internal(client, batch_request).await {
                Ok(response) => return Ok(response),
                Err(e) if !e.is_retryable() => return Err(e),
                Err(e) => {
                    last_error = Some(e);
                    if let Some(retry_after) = last_error.as_ref().and_then(|e| e.retry_after()) {
                        delay = retry_after;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| Error::Api("Max retries exceeded".to_string())))
    }

    /// Send a single batch and handle 207 responses
    async fn send_batch_internal(
        client: &LangfuseClient,
        batch: IngestionBatchRequest,
    ) -> Result<IngestionResponse> {
        // Get event IDs for tracking
        let event_ids: Vec<String> = batch
            .batch
            .iter()
            .map(|event| match event {
                IngestionEvent::IngestionEventOneOf(e) => e.id.clone(),
                IngestionEvent::IngestionEventOneOf1(e) => e.id.clone(),
                IngestionEvent::IngestionEventOneOf2(e) => e.id.clone(),
                IngestionEvent::IngestionEventOneOf3(e) => e.id.clone(),
                IngestionEvent::IngestionEventOneOf4(e) => e.id.clone(),
                IngestionEvent::IngestionEventOneOf5(e) => e.id.clone(),
                IngestionEvent::IngestionEventOneOf6(e) => e.id.clone(),
                _ => uuid::Uuid::new_v4().to_string(),
            })
            .collect();
            

        // Use the raw response API to get status code
        let response = client.configuration.client
            .post(format!("{}/api/public/ingestion", client.base_url))
            .basic_auth(
                &client.public_key,
                Some(&client.secret_key)
            )
            .json(&batch)
            .send()
            .await
            .map_err(|e| Error::Network(e))?;

        let status = response.status();
        let request_id = response.headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Handle different status codes
        match status.as_u16() {
            200 | 201 | 202 => {
                // Full success
                let count = event_ids.len();
                Ok(IngestionResponse {
                    success_ids: event_ids,
                    failures: vec![],
                    success_count: count,
                    failure_count: 0,
                })
            }
            207 => {
                // Multi-Status: Parse the response to identify partial failures
                let body = response.text().await
                    .map_err(|e| Error::Api(format!("Failed to read 207 response: {}", e)))?;
                
                // Parse the 207 response body
                #[derive(serde::Deserialize)]
                struct MultiStatusResponse {
                    successes: Vec<SuccessItem>,
                    errors: Vec<ErrorItem>,
                }
                
                #[derive(serde::Deserialize)]
                struct SuccessItem {
                    id: String,
                    #[allow(dead_code)]
                    status: Option<u16>,
                }
                
                #[derive(serde::Deserialize)]
                struct ErrorItem {
                    id: String,
                    status: Option<u16>,
                    error: Option<String>,
                    message: Option<String>,
                }
                
                let multi_status: MultiStatusResponse = serde_json::from_str(&body)
                    .map_err(|e| Error::Api(format!("Failed to parse 207 response: {}", e)))?;
                
                let success_ids: Vec<String> = multi_status.successes.iter()
                    .map(|s| s.id.clone())
                    .collect();
                    
                let failures: Vec<EventError> = multi_status.errors.iter()
                    .map(|e| EventError {
                        event_id: e.id.clone(),
                        message: e.message.as_ref()
                            .or(e.error.as_ref())
                            .unwrap_or(&"Unknown error".to_string())
                            .clone(),
                        code: e.status.map(|s| s.to_string()),
                        retryable: e.status.map_or(false, |s| s >= 500 || s == 429),
                    })
                    .collect();
                
                Ok(IngestionResponse {
                    success_count: success_ids.len(),
                    failure_count: failures.len(),
                    success_ids,
                    failures,
                })
            }
            401 | 403 => {
                Err(Error::Auth {
                    message: response.text().await.unwrap_or_else(|_| "Authentication failed".to_string()),
                    request_id,
                })
            }
            429 => {
                let retry_after = response.headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(Duration::from_secs);
                    
                Err(Error::RateLimit {
                    retry_after,
                    request_id,
                })
            }
            500..=599 => {
                Err(Error::Server {
                    status: status.as_u16(),
                    message: response.text().await.unwrap_or_else(|_| "Server error".to_string()),
                    request_id,
                })
            }
            _ => {
                Err(Error::Client {
                    status: status.as_u16(),
                    message: response.text().await.unwrap_or_else(|_| format!("Unexpected status: {}", status)),
                    request_id,
                })
            }
        }
    }

    /// Shutdown the batcher and flush remaining events
    pub async fn shutdown(self) -> Result<IngestionResponse> {
        // Signal shutdown
        let _ = self.shutdown_tx.send(()).await;

        // Give time for final flush
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Return final flush result
        self.flush().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_events() {
        let events = vec![
            BatchEvent {
                event: IngestionEvent::IngestionEventOneOf(Box::default()),
                id: "1".to_string(),
                size: 1000,
                retry_count: 0,
            },
            BatchEvent {
                event: IngestionEvent::IngestionEventOneOf(Box::default()),
                id: "2".to_string(),
                size: 2000,
                retry_count: 0,
            },
            BatchEvent {
                event: IngestionEvent::IngestionEventOneOf(Box::default()),
                id: "3".to_string(),
                size: 1500,
                retry_count: 0,
            },
        ];

        // Test size-based chunking
        let chunks = Batcher::chunk_events(&events, 3000, 10);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 2); // First two events fit in 3000 bytes
        assert_eq!(chunks[1].len(), 1); // Last event in separate chunk

        // Test count-based chunking
        let chunks = Batcher::chunk_events(&events, 10000, 2);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 2); // Max 2 events per chunk
        assert_eq!(chunks[1].len(), 1);
    }
}
