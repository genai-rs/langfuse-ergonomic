# Media

Images, audio and documents are uploaded to Langfuse's media storage and referenced from
inputs, outputs or metadata with a token Langfuse renders as the media:
`@@@langfuseMedia:type=image/png|id=<media id>|source=bytes@@@`.

`upload_media()`:

1. reads the upload (a [`timvw_rust_runtime::Upload`](https://docs.rs/timvw-rust-runtime):
   bytes, a stream, or natively a file) and computes its SHA-256;
2. asks Langfuse for an upload URL (`POST /api/public/media`); when Langfuse already has
   these bytes for the field, there is nothing to upload;
3. uploads the bytes (`PUT` to the presigned URL) and reports the result
   (`PATCH /api/public/media/{id}`);
4. returns a `MediaReference`.

```rust,no_run
use langfuse_ergonomic::{ClientBuilder, MediaField};
use serde_json::json;
use timvw_rust_runtime::Upload;

async fn example(png: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::from_env()?.build()?;
    let trace = client.trace().name("vision").call().await?;

    let image = client
        .upload_media()
        .upload(Upload::bytes(png).mime("image/png"))
        .trace_id(&trace.id)
        .field(MediaField::Input)
        .call()
        .await?;

    // Put the reference where the image belongs.
    client
        .generation()
        .trace_id(&trace.id)
        .name("describe-image")
        .input(json!({"prompt": "What is in this picture?", "image": image.reference_string()}))
        .call()
        .await?;

    // Read it back: the record (with a short-lived URL) and the bytes.
    let (media, bytes) = client.download_media(&image.media_id).await?;
    println!("{} bytes of {}", bytes.len(), media.content_type);
    Ok(())
}
```

- The media type comes from `content_type(..)`, else from the upload (`Upload::mime`, or a
  file's extension); Langfuse accepts the types of `MediaContentType`.
- Media belongs to a trace (`trace_id`, with `observation_id` for one observation) or to a
  dataset item (`dataset_id` and `dataset_item_id`). With Langfuse v4 the ids are mapped like
  the builders' ids.
- `MediaReference::parse` reads a token back, and `MediaReference::find_all(&value)` finds
  every token in a JSON value.

See `examples/media.rs`.
