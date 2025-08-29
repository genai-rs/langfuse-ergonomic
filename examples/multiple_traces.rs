//! Example showing multiple related traces in a session

use langfuse_ergonomic::LangfuseClient;
use serde_json::json;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let client = LangfuseClient::from_env()?;
    println!("Connected to Langfuse");

    // Create a session ID to link multiple traces
    let session_id = format!("session-{}", Uuid::new_v4());
    let user_id = "user-123";

    println!("📝 Creating conversation session: {}", session_id);

    // First trace: User asks a question
    let trace1 = client
        .trace()
        .name("chat-message-1")
        .user_id(user_id)
        .session_id(&session_id)
        .input(json!({
            "message": "What's the weather like?"
        }))
        .output(json!({
            "response": "I don't have access to real-time weather data. You might want to check a weather service.",
            "suggested_action": "redirect_to_weather_service"
        }))
        .tags(["chat", "weather-query"])
        .send()
        .await?;

    println!("  Message 1: {}", trace1.id);

    // Second trace: Follow-up question
    let trace2 = client
        .trace()
        .name("chat-message-2")
        .user_id(user_id)
        .session_id(&session_id)
        .input(json!({
            "message": "Can you help me with programming instead?"
        }))
        .output(json!({
            "response": "Of course! I'd be happy to help with programming. What language or topic?",
            "context_switch": true
        }))
        .tags(["chat", "programming-query"])
        .metadata(json!({
            "previous_trace": trace1.id,
            "topic_change": true
        }))
        .send()
        .await?;

    println!("  Message 2: {}", trace2.id);

    // Third trace: Actual programming help
    let trace3 = client
        .trace()
        .name("chat-message-3")
        .user_id(user_id)
        .session_id(&session_id)
        .input(json!({
            "message": "How do I read a file in Rust?",
            "language": "rust"
        }))
        .output(json!({
            "response": "Use std::fs::read_to_string() for UTF-8 text files...",
            "code_example": "let contents = std::fs::read_to_string(\"file.txt\")?;",
            "helpful": true
        }))
        .tags(["chat", "programming-query", "rust", "file-io"])
        .metadata(json!({
            "conversation_depth": 3,
            "topic": "rust_file_operations"
        }))
        .send()
        .await?;

    println!("  Message 3: {}", trace3.id);

    println!("\n✅ Created conversation with 3 messages");
    println!("   Session ID: {}", session_id);
    println!(
        "   View session at: https://cloud.langfuse.com/sessions/{}",
        session_id
    );

    Ok(())
}
