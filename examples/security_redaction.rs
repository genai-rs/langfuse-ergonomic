//! Example demonstrating security features and secret redaction
//!
//! This example shows how to:
//! - Use SecretString for sensitive data
//! - Automatic redaction in Debug/Display
//! - Redact secrets from error messages
//! - Clean sensitive data from JSON payloads

use langfuse_ergonomic::{LangfuseClient, Redactable, Redactor, SecretString};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔒 Demonstrating Security & Secret Redaction\n");
    
    // Example 1: SecretString for API keys
    println!("1️⃣ SecretString protects sensitive values:");
    
    let api_key = SecretString::new("sk-1234567890abcdefghijklmnop");
    
    // Debug and Display automatically redact the value
    println!("   Debug format: {:?}", api_key);
    println!("   Display format: {}", api_key);
    
    // Only expose_secret() reveals the actual value
    println!("   Actual value (use carefully!): {}", api_key.expose_secret());
    println!();
    
    // Example 2: Client with redacted credentials
    println!("2️⃣ Client credentials are automatically protected:");
    
    let client = LangfuseClient::builder()
        .public_key("pk-test-abc123")
        .secret_key("sk-secret-xyz789")
        .base_url("https://cloud.langfuse.com".to_string())
        .build();
    
    // Debug output redacts the keys
    println!("   Client debug: {:?}", client);
    println!();
    
    // Example 3: Redacting sensitive data from strings
    println!("3️⃣ Redacting various secret patterns:");
    
    let text_with_secrets = r#"
        API_KEY=sk-1234567890abcdefghijklmnop
        Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U
        Database: postgresql://user:password123@localhost:5432/mydb
        AWS_ACCESS_KEY_ID=AKIA1234567890ABCDEF
        Email: user@example.com
        SSN: 123-45-6789
        Credit Card: 1234-5678-9012-3456
    "#;
    
    let redacted = text_with_secrets.redacted();
    println!("   Original contains secrets? Yes");
    println!("   Redacted version:");
    for line in redacted.lines() {
        if !line.trim().is_empty() {
            println!("     {}", line);
        }
    }
    println!();
    
    // Example 4: Redacting JSON data
    println!("4️⃣ Redacting sensitive data from JSON:");
    
    let json_with_secrets = json!({
        "user": {
            "name": "John Doe",
            "email": "john@example.com",
            "api_key": "sk-super-secret-key-123",
            "password": "MyP@ssw0rd!",
            "public_info": "This is public information"
        },
        "config": {
            "database_url": "postgres://admin:secretpass@db.example.com:5432/myapp",
            "auth_token": "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
            "debug_mode": true
        },
        "metadata": {
            "version": "1.0.0",
            "environment": "production"
        }
    });
    
    println!("   Original JSON contains sensitive fields");
    let redacted_json = json_with_secrets.redacted();
    println!("   Redacted JSON:");
    for line in redacted_json.lines() {
        println!("     {}", line);
    }
    println!();
    
    // Example 5: Custom redaction patterns
    println!("5️⃣ Adding custom redaction patterns:");
    
    let mut redactor = Redactor::new();
    
    // Add custom pattern for internal IDs
    redactor.add_pattern(
        "internal_id",
        r"INT-[A-Z0-9]{8}",
        "INT-***REDACTED***"
    )?;
    
    // Add custom pattern for phone numbers
    redactor.add_pattern(
        "phone",
        r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b",
        "***PHONE***"
    )?;
    
    let text_with_custom = "Internal ID: INT-ABC12345, Phone: 555-123-4567, Public: ABC123";
    let custom_redacted = redactor.redact(text_with_custom);
    
    println!("   Original: {}", text_with_custom);
    println!("   Redacted: {}", custom_redacted);
    println!();
    
    // Example 6: Error message redaction
    println!("6️⃣ Automatic error redaction:");
    
    // Simulate an error with sensitive information
    let error_msg = "Failed to connect to database://admin:SuperSecret123@prod.db.com:5432";
    let api_error = langfuse_ergonomic::Error::Api(error_msg.to_string());
    
    // Debug format automatically redacts sensitive data
    println!("   Error (Debug): {:?}", api_error);
    
    // Display format shows the error message (also redacted internally)
    println!("   Error (Display): {}", api_error);
    println!();
    
    // Example 7: Trace creation with automatic redaction
    println!("7️⃣ Traces with sensitive data are protected:");
    
    // This would normally create a trace, but we'll simulate the data structure
    let trace_input = json!({
        "user_prompt": "Process payment for user",
        "internal_api_key": "sk-internal-key-xyz",
        "credit_card": "4111-1111-1111-1111",
        "safe_data": "This is safe to log"
    });
    
    println!("   Original trace input contains sensitive data");
    println!("   Would be redacted before sending to Langfuse:");
    let redacted_input = trace_input.redacted();
    for line in redacted_input.lines().take(5) {
        println!("     {}", line);
    }
    println!("     ...");
    println!();
    
    println!("✅ Security features demonstration complete!");
    println!("\n📝 Summary:");
    println!("   - SecretString protects API keys and passwords");
    println!("   - Automatic redaction in Debug/Display implementations");
    println!("   - Built-in patterns for common secrets (API keys, JWTs, etc.)");
    println!("   - Custom patterns can be added for domain-specific secrets");
    println!("   - JSON values are recursively redacted");
    println!("   - Error messages automatically redact sensitive information");
    
    Ok(())
}