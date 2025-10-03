//! Example demonstrating observation tracking (spans, generations, events)

use langfuse_ergonomic::ClientBuilder;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client from environment variables
    let client = ClientBuilder::from_env()?.build()?;

    // Create a trace to group our observations
    let trace = client
        .trace()
        .name("llm-application-flow")
        .user_id("user-123")
        .session_id("session-456")
        .metadata(json!({
            "environment": "production",
            "version": "1.0.0"
        }))
        .call()
        .await?;

    println!("Created trace: {}", trace.id);

    // Create a span for the overall operation
    let main_span_id = client
        .span()
        .trace_id(&trace.id)
        .name("process-user-query")
        .input(json!({"query": "What is the weather like?"}))
        .level("INFO")
        .call()
        .await?;

    println!("Created main span: {}", main_span_id);

    // Create a nested span for preprocessing
    let preprocessing_span_id = client
        .span()
        .trace_id(&trace.id)
        .parent_observation_id(&main_span_id)
        .name("preprocess-query")
        .input(json!({"raw_query": "What is the weather like?"}))
        .output(json!({"processed_query": "weather current location"}))
        .call()
        .await?;

    println!("Created preprocessing span: {}", preprocessing_span_id);

    // Log an event for an important milestone
    let event_id = client
        .event()
        .trace_id(&trace.id)
        .parent_observation_id(&main_span_id)
        .name("cache-check")
        .input(json!({"cache_key": "weather_current"}))
        .output(json!({"cache_hit": false}))
        .level("DEBUG")
        .call()
        .await?;

    println!("Created event: {}", event_id);

    // Create a generation for the LLM call
    let generation_id = client
        .generation()
        .trace_id(&trace.id)
        .parent_observation_id(&main_span_id)
        .name("llm-completion")
        .model("gpt-4")
        .input(json!({
            "messages": [
                {"role": "system", "content": "You are a helpful weather assistant."},
                {"role": "user", "content": "What is the weather like?"}
            ]
        }))
        .output(json!({
            "content": "I'd be happy to help you with weather information. However, I need to know your location to provide accurate weather details. Could you please tell me which city or area you're interested in?"
        }))
        .prompt_tokens(50)
        .completion_tokens(45)
        .metadata(json!({
            "temperature": 0.7,
            "max_tokens": 150
        }))
        .call()
        .await?;

    println!("Created generation: {}", generation_id);

    // Create another event for post-processing
    let postprocess_event_id = client
        .event()
        .trace_id(&trace.id)
        .parent_observation_id(&main_span_id)
        .name("response-validation")
        .input(json!({"response_length": 95}))
        .output(json!({"valid": true, "requires_followup": true}))
        .level("INFO")
        .status_message("Response validated successfully")
        .call()
        .await?;

    println!("Created post-processing event: {}", postprocess_event_id);

    // Log an error event example
    let error_event_id = client
        .event()
        .trace_id(&trace.id)
        .name("rate-limit-warning")
        .level("WARNING")
        .status_message("Approaching rate limit: 95% of quota used")
        .metadata(json!({
            "requests_remaining": 50,
            "reset_time": "2024-01-01T00:00:00Z"
        }))
        .call()
        .await?;

    println!("Created warning event: {}", error_event_id);

    println!("\nAll observations created successfully!");
    println!("View them in Langfuse dashboard for trace ID: {}", trace.id);

    Ok(())
}
