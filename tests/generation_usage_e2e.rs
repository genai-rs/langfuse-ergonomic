//! Live check that generation token usage persists in Langfuse.
//!
//! Ignored by default. `scripts/live-tests.sh` starts a self-hosted Langfuse and runs it for
//! both API versions; against another server set `LANGFUSE_PUBLIC_KEY`,
//! `LANGFUSE_SECRET_KEY`, `LANGFUSE_BASE_URL` and `LANGFUSE_API_VERSION` (`v3` or `v4`) and
//! run `cargo test --test generation_usage_e2e -- --ignored --nocapture`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{live_client, wait_for_observation};

#[tokio::test]
#[ignore = "requires a Langfuse server; run scripts/live-tests.sh"]
async fn generation_token_usage_persists_in_langfuse() {
    let client = live_client();
    let trace = client
        .trace()
        .name(format!("issue-115-e2e-{}", uuid::Uuid::new_v4()))
        .tags(vec!["e2e".to_owned(), "issue-115".to_owned()])
        .call()
        .await
        .expect("create live test trace");
    println!("Live test trace: {}", trace.url());

    // The first case is the reported failure: total is omitted by the caller
    // and must be derived by Langfuse after ingesting the supplied counts.
    let cases = [
        ("reported-counts", Some(1200), Some(340), None, 1540),
        ("explicit-total", Some(1200), Some(340), Some(1600), 1600),
        ("input-only", Some(42), None, None, 42),
        ("total-only", None, None, Some(77), 77),
        ("zero-counts", Some(0), Some(0), Some(0), 0),
    ];

    for (name, input, output, total, expected_total) in cases {
        let id = client
            .generation()
            .trace_id(&trace.id)
            .name(name)
            .model("gpt-4")
            .maybe_prompt_tokens(input)
            .maybe_completion_tokens(output)
            .maybe_total_tokens(total)
            .call()
            .await
            .expect("ingest live generation");

        let mut expected = vec![("total", i64::from(expected_total))];
        if let Some(input) = input {
            expected.push(("input", i64::from(input)));
        }
        if let Some(output) = output {
            expected.push(("output", i64::from(output)));
        }
        let observation = wait_for_observation(&client, &trace.id, &id, |o| {
            if expected
                .iter()
                .all(|(key, value)| o.usage_details.get(*key) == Some(value))
            {
                Ok(())
            } else {
                Err(format!("expected usage {expected:?}"))
            }
        })
        .await;
        assert_eq!(observation.id, id);
        assert_eq!(observation.trace_id.as_deref(), Some(trace.id.as_str()));
        assert_eq!(observation.r#type, "GENERATION");
        println!(
            "Verified {name}: observation={id}, usageDetails={:?}",
            observation.usage_details
        );
    }
}
