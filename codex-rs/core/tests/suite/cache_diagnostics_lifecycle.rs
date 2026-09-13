use std::time::Duration;

use anyhow::Result;
use codex_core::TurnInputRequest;
use codex_model_provider_info::WireApi;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_once;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::sse_failed;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::streaming_sse::StreamingSseChunk;
use core_test_support::streaming_sse::start_streaming_sse_server;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use serde_json::json;
use tokio::sync::oneshot;
use tokio::time::timeout;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::http::Method;
use wiremock::matchers::method;
use wiremock::matchers::path_regex;

use super::UsageFixture;
use super::completed_with_usage;
use super::read_diagnostic_archive;
use super::usage_json;

const LOCAL_COMPACTION_CANARY: &str = "PRIVATE-DIAGNOSTIC-LOCAL-COMPACTION-CANARY";
const LOCAL_COMPACTION_PROMPT: &str = "PRIVATE-DIAGNOSTIC-LOCAL-COMPACTION-PROMPT";
const LOCAL_COMPACTION_ERROR: &str = "PRIVATE-DIAGNOSTIC-LOCAL-COMPACTION-STREAM-ERROR";
const RETRY_CANARY: &str = "PRIVATE-DIAGNOSTIC-RETRY-CANARY";
const RETRY_FAILURE_CANARY: &str = "PRIVATE-DIAGNOSTIC-RETRY-FAILURE";
const FAILED_RESPONSE_ID: &str = "private-diagnostic-failed-response";
const RETRY_RESPONSE_ID: &str = "private-diagnostic-retry-response";
const FALLBACK_RESPONSE_ID: &str = "response-after-upgrade-required";
const INITIAL_USAGE: UsageFixture = UsageFixture {
    input: 11,
    cached_input: 3,
    cache_write: 2,
    output: 5,
    reasoning: 1,
    total: 16,
};
const RETRY_USAGE: UsageFixture = UsageFixture {
    input: 307,
    cached_input: 257,
    cache_write: 31,
    output: 41,
    reasoning: 37,
    total: 348,
};
const LOCAL_COMPACTION_USAGE: UsageFixture = UsageFixture {
    input: 401,
    cached_input: 331,
    cache_write: 43,
    output: 47,
    reasoning: 11,
    total: 448,
};
const FALLBACK_USAGE: UsageFixture = UsageFixture {
    input: 17,
    cached_input: 7,
    cache_write: 3,
    output: 5,
    reasoning: 2,
    total: 22,
};

fn assert_bytes_exclude(bytes: &[u8], canaries: &[&str]) {
    for canary in canaries {
        assert!(
            !bytes
                .windows(canary.len())
                .any(|window| window == canary.as_bytes()),
            "diagnostic archive exposed private canary {canary}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_compaction_failed_stream_retry_records_ordinals_zero_and_one() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let response_mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("before-local-compaction-response"),
                ev_assistant_message("before-local-compaction-message", "context to compact"),
                completed_with_usage("before-local-compaction-response", INITIAL_USAGE),
            ]),
            sse_failed(
                "failed-local-compaction-response",
                "server_error",
                LOCAL_COMPACTION_ERROR,
            ),
            sse(vec![
                ev_response_created("completed-local-compaction-response"),
                ev_assistant_message("local-compaction-summary", "compacted context"),
                completed_with_usage(
                    "completed-local-compaction-response",
                    LOCAL_COMPACTION_USAGE,
                ),
            ]),
        ],
    )
    .await;
    let mut builder = test_codex().with_config(|config| {
        config.model_provider.name = "OpenAI-compatible local compaction test".to_string();
        config.model_provider.supports_websockets = false;
        config.model_provider.stream_max_retries = Some(1);
        config.compact_prompt = Some(LOCAL_COMPACTION_PROMPT.to_string());
    });
    let test = builder.build_with_auto_env(&server).await?;

    test.submit_turn(LOCAL_COMPACTION_CANARY).await?;
    test.codex.submit(Op::Compact).await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::StreamError(_))
    })
    .await;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    test.codex.shutdown_and_wait().await?;

    let outbound = response_mock.requests();
    assert_eq!(outbound.len(), 3);
    let archive = read_diagnostic_archive(test.home.path());
    assert_eq!(archive.records.len(), 6);
    assert_eq!(archive.records[0]["requestKind"], "turn");
    let first_request = &archive.records[2];
    let first_outcome = &archive.records[3];
    let retry_request = &archive.records[4];
    let retry_outcome = &archive.records[5];
    for (request, outbound) in [(first_request, &outbound[1]), (retry_request, &outbound[2])] {
        assert_eq!(request["requestKind"], "compaction");
        assert_eq!(request["wire"]["kind"], "http");
        assert_eq!(
            request["wire"]["body"]["bytes"],
            outbound.body_bytes().len()
        );
        assert_eq!(
            request["logical"]["body"]["bytes"],
            serde_json::to_vec(&outbound.body_json())?.len()
        );
        assert_eq!(request["wire"]["transport"]["endpoint"], "responses");
    }
    assert_eq!(first_request["retryOrdinal"], 0);
    assert_eq!(retry_request["retryOrdinal"], 1);
    assert_ne!(first_request["attemptId"], retry_request["attemptId"]);
    assert_eq!(first_request["attemptId"], first_outcome["attemptId"]);
    assert_eq!(first_outcome["terminal"], "failed");
    assert_eq!(first_outcome["usage"], json!(null));
    assert_eq!(retry_request["attemptId"], retry_outcome["attemptId"]);
    assert_eq!(retry_outcome["terminal"], "completed");
    assert_eq!(retry_outcome["usage"], usage_json(LOCAL_COMPACTION_USAGE));
    assert_eq!(outbound[1].body_json(), outbound[2].body_json());
    // Equivalent client metadata maps can serialize in different member orders.
    let stable_manifest = |request: &serde_json::Value| {
        let mut logical = request["logical"].as_object().unwrap().clone();
        for component in ["body", "clientMetadata"] {
            let fingerprint = logical.remove(component).unwrap();
            logical.insert(component.to_string(), fingerprint["bytes"].clone());
        }
        logical
    };
    assert_eq!(
        stable_manifest(first_request),
        stable_manifest(retry_request)
    );
    assert_eq!(
        first_request["wire"]["body"]["bytes"],
        retry_request["wire"]["body"]["bytes"]
    );
    assert_bytes_exclude(
        &archive.bytes,
        &[
            LOCAL_COMPACTION_CANARY,
            LOCAL_COMPACTION_PROMPT,
            LOCAL_COMPACTION_ERROR,
            "failed-local-compaction-response",
            "completed-local-compaction-response",
        ],
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_stream_retry_records_each_actual_send_with_the_next_ordinal() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let response_mock = mount_sse_sequence(
        &server,
        vec![
            sse_failed(
                FAILED_RESPONSE_ID,
                "rate_limit_exceeded",
                RETRY_FAILURE_CANARY,
            ),
            sse(vec![
                ev_response_created(RETRY_RESPONSE_ID),
                ev_assistant_message("retry-recovered-message", "done"),
                completed_with_usage(RETRY_RESPONSE_ID, RETRY_USAGE),
            ]),
        ],
    )
    .await;
    let test = test_codex()
        .with_config(|config| {
            config.model_provider.request_max_retries = Some(0);
            config.model_provider.stream_max_retries = Some(1);
        })
        .build_with_auto_env(&server)
        .await?;

    test.submit_turn(RETRY_CANARY).await?;
    test.codex.shutdown_and_wait().await?;

    let outbound = response_mock.requests();
    assert_eq!(outbound.len(), 2);
    let archive = read_diagnostic_archive(test.home.path());
    assert_eq!(archive.records.len(), 4);
    let first_request = &archive.records[0];
    let first_outcome = &archive.records[1];
    let second_request = &archive.records[2];
    let second_outcome = &archive.records[3];
    assert_eq!(first_request["requestKind"], "turn");
    assert_eq!(first_request["retryOrdinal"], 0);
    assert_eq!(first_request["wire"]["kind"], "http");
    assert_eq!(
        first_request["wire"]["body"]["bytes"],
        outbound[0].body_bytes().len()
    );
    assert_eq!(first_request["attemptId"], first_outcome["attemptId"]);
    assert_eq!(first_outcome["terminal"], "failed");
    assert_eq!(first_outcome["responseId"], json!({"status": "missing"}));
    assert_eq!(first_outcome["usage"], json!(null));
    assert_eq!(second_request["requestKind"], "turn");
    assert_eq!(second_request["retryOrdinal"], 1);
    assert_eq!(second_request["wire"]["kind"], "http");
    assert_eq!(
        second_request["wire"]["body"]["bytes"],
        outbound[1].body_bytes().len()
    );
    assert_ne!(first_request["attemptId"], second_request["attemptId"]);
    assert_eq!(second_request["attemptId"], second_outcome["attemptId"]);
    assert_eq!(second_outcome["terminal"], "completed");
    assert_eq!(second_outcome["usage"], usage_json(RETRY_USAGE));
    assert!(second_outcome["responseId"]["hmac"].is_string());
    assert_bytes_exclude(
        &archive.bytes,
        &[
            RETRY_CANARY,
            RETRY_FAILURE_CANARY,
            FAILED_RESPONSE_ID,
            RETRY_RESPONSE_ID,
        ],
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upgrade_required_before_websocket_send_records_only_http_fallback() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    Mock::given(method("GET"))
        .and(path_regex(".*/responses$"))
        .respond_with(ResponseTemplate::new(/*status*/ 426))
        .mount(&server)
        .await;
    let response_mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created(FALLBACK_RESPONSE_ID),
            ev_assistant_message("message-after-upgrade-required", "done"),
            completed_with_usage(FALLBACK_RESPONSE_ID, FALLBACK_USAGE),
        ]),
    )
    .await;
    let base_url = format!("{}/v1", server.uri());
    let mut builder = test_codex().with_config(move |config| {
        config.model_provider.base_url = Some(base_url);
        config.model_provider.wire_api = WireApi::Responses;
        config.model_provider.supports_websockets = true;
        config.model_provider.stream_max_retries = Some(2);
        config.model_provider.request_max_retries = Some(0);
    });
    let test = builder.build_with_auto_env(&server).await?;

    test.submit_turn("use HTTP after upgrade required").await?;
    test.codex.shutdown_and_wait().await?;

    let received = server.received_requests().await.unwrap_or_default();
    assert_eq!(
        received
            .iter()
            .filter(|request| request.method == Method::GET)
            .count(),
        1
    );
    assert_eq!(
        received
            .iter()
            .filter(|request| request.method == Method::POST)
            .count(),
        1
    );
    let outbound = response_mock.single_request();
    let archive = read_diagnostic_archive(test.home.path());
    assert_eq!(archive.records.len(), 2);
    let request = &archive.records[0];
    let outcome = &archive.records[1];
    assert_eq!(request["requestKind"], "turn");
    assert_eq!(request["retryOrdinal"], 0);
    assert_eq!(request["wire"]["kind"], "http");
    assert_eq!(
        request["wire"]["body"]["bytes"],
        outbound.body_bytes().len()
    );
    assert_eq!(request["wire"]["transport"]["endpoint"], "responses");
    assert_eq!(request["attemptId"], outcome["attemptId"]);
    assert_eq!(outcome["terminal"], "completed");
    assert_eq!(outcome["usage"], usage_json(FALLBACK_USAGE));
    assert!(outcome["responseId"]["hmac"].is_string());
    assert_bytes_exclude(&archive.bytes, &[FALLBACK_RESPONSE_ID]);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interrupted_stream_records_exactly_one_linked_cancelled_outcome() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let (_release_response, response_gate) = oneshot::channel();
    let (server, _completions) = start_streaming_sse_server(vec![vec![
        StreamingSseChunk {
            gate: None,
            body: sse(vec![ev_response_created("interrupted-diagnostic-response")]),
        },
        StreamingSseChunk {
            gate: Some(response_gate),
            body: sse(vec![ev_completed("interrupted-diagnostic-response")]),
        },
    ]])
    .await;
    let mut builder = test_codex();
    let test = builder.build_with_streaming_server(&server).await?;

    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "interrupt this diagnostic stream".to_string(),
            text_elements: Vec::new(),
        }]))
        .await?;
    timeout(
        Duration::from_secs(5),
        server.wait_for_request_count(/*count*/ 1),
    )
    .await
    .expect("interrupted stream should reach the model request");
    test.codex.submit(Op::Interrupt).await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnAborted(_))
    })
    .await;
    test.codex.shutdown_and_wait().await?;

    let outbound = server.requests().await;
    assert_eq!(outbound.len(), 1);
    server.shutdown().await;
    let archive = read_diagnostic_archive(test.home.path());
    assert_eq!(archive.records.len(), 2);
    let request = &archive.records[0];
    let outcome = &archive.records[1];
    assert_eq!(request["event"], "request");
    assert_eq!(request["requestKind"], "turn");
    assert_eq!(request["retryOrdinal"], 0);
    assert_eq!(request["wire"]["kind"], "http");
    assert_eq!(request["wire"]["body"]["bytes"], outbound[0].len());
    assert_eq!(request["attemptId"], outcome["attemptId"]);
    assert_eq!(outcome["event"], "outcome");
    assert_eq!(outcome["terminal"], "cancelled");
    assert_eq!(outcome["responseId"], json!({"status": "missing"}));
    assert_eq!(outcome["usage"], json!(null));
    assert_bytes_exclude(
        &archive.bytes,
        &[
            "interrupt this diagnostic stream",
            "interrupted-diagnostic-response",
        ],
    );

    Ok(())
}
