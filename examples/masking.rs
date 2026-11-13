//! A mask function that redacts e-mail addresses and passwords from everything sent.
//!
//! `cargo run --example masking`
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use langfuse_ergonomic::{observe, ClientBuilder};
use serde_json::{json, Value};

fn redact(value: Value) -> Value {
    match value {
        Value::String(text) if text.contains('@') => Value::String("[email]".to_owned()),
        Value::Array(items) => Value::Array(items.into_iter().map(redact).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    let value = if key == "password" {
                        Value::from("***")
                    } else {
                        redact(value)
                    };
                    (key, value)
                })
                .collect(),
        ),
        other => other,
    }
}

#[derive(serde::Serialize)]
struct Login {
    email: String,
    password: String,
}

#[observe]
async fn login(request: Login) -> bool {
    !request.email.is_empty() && !request.password.is_empty()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let client = ClientBuilder::from_env()?.mask(redact).build()?;

    // The builders are masked...
    let trace = client
        .trace()
        .name("masking-example")
        .input(json!({"email": "ada@example.com", "password": "hunter2"}))
        .call()
        .await?;

    // ...and so are batches and #[observe] observations.
    let batcher = client.batcher().build().await;
    observe::install_batcher(batcher.clone());
    login(Login {
        email: "grace@example.com".to_owned(),
        password: "correct horse".to_owned(),
    })
    .await;
    batcher.shutdown().await?;

    println!("Trace: {}", trace.url());
    Ok(())
}
