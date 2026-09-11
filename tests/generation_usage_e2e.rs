//! Live Langfuse verification. Run explicitly with LANGFUSE_PUBLIC_KEY,
//! LANGFUSE_SECRET_KEY, and LANGFUSE_BASE_URL configured:
//! cargo test --test generation_usage_e2e -- --ignored --nocapture

use langfuse_ergonomic::{ClientBuilder, LangfuseClient};
use std::time::Duration;
use tokio::time::{sleep, timeout, Instant};

async fn wait_for_usage(
    client: &LangfuseClient,
    observation_id: &str,
    expected: &[(&str, i32)],
) -> langfuse_client_base::models::ObservationsView {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let diagnostic = match timeout(
            Duration::from_secs(10),
            client.get_observation(observation_id),
        )
        .await
        {
            Ok(Ok(observation)) => {
                if expected
                    .iter()
                    .all(|(key, value)| observation.usage_details.get(*key) == Some(value))
                {
                    return observation;
                }
                format!("stored usageDetails: {:?}", observation.usage_details)
            }
            Ok(Err(error)) => format!("readback error: {error}"),
            Err(_) => "readback request timed out".to_owned(),
        };

        assert!(
            Instant::now() < deadline,
            "Observation {observation_id} did not persist expected usage {expected:?}: {diagnostic}"
        );
        sleep(Duration::from_secs(2)).await;
    }
}

#[tokio::test]
#[ignore = "writes synthetic traces to a real Langfuse instance; requires API credentials"]
async fn generation_token_usage_persists_in_langfuse() {
    let client = ClientBuilder::from_env()
        .and_then(|builder| builder.build())
        .expect("configure Langfuse test-project credentials");
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

        let mut expected = vec![("total", expected_total)];
        if let Some(input) = input {
            expected.push(("input", input));
        }
        if let Some(output) = output {
            expected.push(("output", output));
        }
        let observation = wait_for_usage(&client, &id, &expected).await;
        assert_eq!(observation.id, id);
        assert_eq!(observation.trace_id, Some(Some(trace.id.clone())));
        assert_eq!(observation.r#type, "GENERATION");
        assert_eq!(observation.usage.total, expected_total);
        if let Some(input) = input {
            assert_eq!(observation.usage.input, input);
        }
        if let Some(output) = output {
            assert_eq!(observation.usage.output, output);
        }
        println!(
            "Verified {name}: observation={id}, usageDetails={:?}, costDetails={:?}",
            observation.usage_details, observation.cost_details
        );
    }
}
