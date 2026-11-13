//! Upload an image, reference it from a generation's input, and download it again.
//!
//! `cargo run --example media`
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use langfuse_ergonomic::{ClientBuilder, MediaField};
use serde_json::json;
use timvw_rust_runtime::Upload;

/// A 1x1 transparent PNG.
const PIXEL: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae,
    0x42, 0x60, 0x82,
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let client = ClientBuilder::from_env()?.build()?;
    let trace = client.trace().name("media-example").call().await?;

    let image = client
        .upload_media()
        .upload(Upload::bytes(PIXEL).mime("image/png"))
        .trace_id(&trace.id)
        .field(MediaField::Input)
        .call()
        .await?;
    println!("Uploaded {}", image.reference_string());

    client
        .generation()
        .trace_id(&trace.id)
        .name("describe-image")
        .input(json!({"prompt": "Describe this image", "image": image.reference_string()}))
        .output(json!("A single transparent pixel."))
        .call()
        .await?;

    let (media, bytes) = client.download_media(&image.media_id).await?;
    println!("Downloaded {} bytes of {}", bytes.len(), media.content_type);
    println!("Trace: {}", trace.url());
    Ok(())
}
