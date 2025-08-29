//! Example demonstrating score tracking and evaluation

use langfuse_ergonomic::LangfuseClient;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client from environment variables
    let client = LangfuseClient::from_env()?;

    // Create a trace for an LLM interaction
    let trace = client
        .trace()
        .name("chatbot-conversation")
        .user_id("user-789")
        .session_id("chat-session-123")
        .input(json!({
            "message": "Can you help me write a Python function to sort a list?"
        }))
        .output(json!({
            "response": "Of course! Here's a simple Python function to sort a list:\n\n```python\ndef sort_list(lst):\n    return sorted(lst)\n```"
        }))
        .metadata(json!({
            "conversation_turn": 1,
            "topic": "programming"
        }))
        .send()
        .await?;

    println!("Created trace: {}", trace.id);

    // Add a numeric score for response quality
    let quality_score_id = client
        .score(&trace.id, "response_quality")
        .value(0.85)
        .comment("Good response with code example")
        .metadata(json!({
            "evaluated_by": "automated_scorer",
            "criteria": ["relevance", "completeness", "code_quality"]
        }))
        .send()
        .await?;

    println!("Created quality score: {}", quality_score_id);

    // Add a categorical score for sentiment
    let sentiment_score_id = client
        .score(&trace.id, "user_sentiment")
        .string_value("positive")
        .comment("User expressed satisfaction")
        .send()
        .await?;

    println!("Created sentiment score: {}", sentiment_score_id);

    // Use the binary score helper for a success/failure metric
    let success_score_id = client
        .binary_score(&trace.id, "task_completed", true)
        .comment("User's request was successfully addressed")
        .send()
        .await?;

    println!("Created binary score: {}", success_score_id);

    // Use the rating score helper for user feedback
    let rating_score_id = client
        .rating_score(&trace.id, "user_rating", 4, 5)
        .comment("User gave 4 out of 5 stars")
        .metadata(json!({
            "feedback_prompt": "How helpful was this response?"
        }))
        .send()
        .await?;

    println!("Created rating score: {}", rating_score_id);

    // Use categorical score helper for classification
    let category_score_id = client
        .categorical_score(&trace.id, "response_type", "code_generation")
        .comment("Response included code generation")
        .send()
        .await?;

    println!("Created categorical score: {}", category_score_id);

    // Create a generation and score it
    let generation_id = client
        .generation(&trace.id)
        .name("code-generation")
        .model("gpt-4")
        .input(json!({"prompt": "Write a Python function to sort a list"}))
        .output(json!({"code": "def sort_list(lst):\n    return sorted(lst)"}))
        .tokens(15, 20)
        .send()
        .await?;

    println!("Created generation: {}", generation_id);

    // Score the specific generation (observation-level score)
    let generation_score_id = client
        .score(&trace.id, "code_correctness")
        .observation_id(&generation_id)
        .value(1.0)
        .comment("Generated code is syntactically correct and functional")
        .send()
        .await?;

    println!("Created generation-specific score: {}", generation_score_id);

    // Example of multiple evaluation criteria
    let criteria = vec![
        ("accuracy", 0.9, "Factually correct"),
        ("helpfulness", 0.85, "Addressed user's needs"),
        ("safety", 1.0, "No harmful content"),
        ("coherence", 0.95, "Well-structured response"),
    ];

    for (name, value, comment) in criteria {
        let score_id = client
            .score(&trace.id, name)
            .value(value)
            .comment(comment)
            .send()
            .await?;
        println!("Created {} score: {}", name, score_id);
    }

    println!("\nAll scores created successfully!");
    println!("View them in Langfuse dashboard for trace ID: {}", trace.id);

    Ok(())
}
