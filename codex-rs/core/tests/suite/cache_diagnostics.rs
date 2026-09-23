//! Verifies default-on cache diagnostics across the real core request path.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_once;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::responses::start_websocket_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;
use walkdir::WalkDir;

const USER_CANARY: &str = "PRIVATE-DIAGNOSTIC-USER-CANARY";
const TOOL_CANARY: &str = "PRIVATE-DIAGNOSTIC-TOOL-CANARY";
const WARM_RESPONSE_ID: &str = "private-diagnostic-warm-response";
const FIRST_RESPONSE_ID: &str = "private-diagnostic-response-one";
const SECOND_RESPONSE_ID: &str = "private-diagnostic-response-two";

#[derive(Clone, Copy)]
struct UsageFixture {
    input: i64,
    cached_input: i64,
    cache_write: i64,
    output: i64,
    reasoning: i64,
    total: i64,
}

const FIRST_USAGE: UsageFixture = UsageFixture {
    input: 101,
    cached_input: 71,
    cache_write: -13,
    output: 23,
    reasoning: 17,
    total: 124,
};
const SECOND_USAGE: UsageFixture = UsageFixture {
    input: 211,
    cached_input: 181,
    cache_write: 19,
    output: 29,
    reasoning: 7,
    total: 240,
};
const ZERO_USAGE: UsageFixture = UsageFixture {
    input: 0,
    cached_input: 0,
    cache_write: 0,
    output: 0,
    reasoning: 0,
    total: 0,
};

struct DiagnosticArchive {
    bytes: Vec<u8>,
    records: Vec<Value>,
}

fn completed_with_usage(id: &str, usage: UsageFixture) -> Value {
    json!({
        "type": "response.completed",
        "response": {
            "id": id,
            "usage": {
                "input_tokens": usage.input,
                "input_tokens_details": {
                    "cached_tokens": usage.cached_input,
                    "cache_write_tokens": usage.cache_write,
                },
                "output_tokens": usage.output,
                "output_tokens_details": { "reasoning_tokens": usage.reasoning },
                "total_tokens": usage.total,
            },
        },
    })
}

fn usage_json(usage: UsageFixture) -> Value {
    json!({
        "input": usage.input,
        "cachedInput": usage.cached_input,
        "cacheWrite": usage.cache_write,
        "output": usage.output,
        "reasoning": usage.reasoning,
        "total": usage.total,
    })
}

fn two_response_tool_sequence() -> Vec<Vec<Value>> {
    let plan = json!({
        "plan": [{
            "step": TOOL_CANARY,
            "status": "in_progress",
        }],
    })
    .to_string();
    vec![
        vec![
            ev_response_created(FIRST_RESPONSE_ID),
            ev_function_call("diagnostic-call", "update_plan", &plan),
            completed_with_usage(FIRST_RESPONSE_ID, FIRST_USAGE),
        ],
        vec![
            ev_response_created(SECOND_RESPONSE_ID),
            ev_assistant_message("diagnostic-message", "done"),
            completed_with_usage(SECOND_RESPONSE_ID, SECOND_USAGE),
        ],
    ]
}

fn read_diagnostic_archive(home: &Path) -> DiagnosticArchive {
    let mut paths = WalkDir::new(home.join("cache-diagnostics"))
        .into_iter()
        .map(|entry| entry.expect("cache diagnostic path should be readable"))
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|extension| extension.to_str())
                    == Some("jsonl")
        })
        .map(walkdir::DirEntry::into_path)
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(paths.len(), 1, "expected one collector run file");

    let bytes = fs::read(&paths[0]).expect("cache diagnostic run file should be readable");
    let records = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).expect("diagnostic line should be valid JSON"))
        .collect();
    DiagnosticArchive { bytes, records }
}

fn assert_two_linked_attempts(
    records: &[Value],
    first_sequence: u64,
    wire_kind: &str,
    wire_sizes: &[usize; 2],
    logical_sizes: Option<&[usize; 2]>,
    retry_ordinals: &[u64; 2],
) {
    assert_eq!(records.len(), 4);
    let run_id = records[0]["runId"]
        .as_str()
        .expect("diagnostic record should carry a run id");
    assert!(!run_id.is_empty());
    for record in records {
        assert_eq!(record["schemaVersion"], 2);
        assert_eq!(record["runId"], run_id);
        assert!(
            record["attemptId"]
                .as_str()
                .is_some_and(|id| !id.is_empty())
        );
    }
    assert_eq!(
        records
            .iter()
            .map(|record| (record["sequence"].as_u64(), record["event"].as_str()))
            .collect::<Vec<_>>(),
        vec![
            (Some(first_sequence), Some("request")),
            (Some(first_sequence + 1), Some("outcome")),
            (Some(first_sequence + 2), Some("request")),
            (Some(first_sequence + 3), Some("outcome")),
        ]
    );

    let requests = [&records[0], &records[2]];
    let outcomes = [&records[1], &records[3]];
    assert_ne!(requests[0]["attemptId"], requests[1]["attemptId"]);
    assert_ne!(
        requests[0]["logical"]["body"],
        requests[1]["logical"]["body"]
    );
    assert_ne!(requests[0]["wire"]["body"], requests[1]["wire"]["body"]);

    for (index, request) in requests.into_iter().enumerate() {
        assert_eq!(request["requestKind"], "turn");
        assert_eq!(request["retryOrdinal"], retry_ordinals[index]);
        assert_eq!(request["wire"]["kind"], wire_kind);
        assert_eq!(request["wire"]["body"]["bytes"], wire_sizes[index]);
        assert!(request["logical"]["body"]["hmac"].is_string());
        assert!(request["wire"]["body"]["hmac"].is_string());
        if let Some(logical_sizes) = logical_sizes {
            assert_eq!(request["logical"]["body"]["bytes"], logical_sizes[index]);
        }
    }

    for (request, outcome) in requests.into_iter().zip(outcomes) {
        assert_eq!(request["attemptId"], outcome["attemptId"]);
        assert_eq!(outcome["terminal"], "completed");
        assert!(outcome["responseId"]["hmac"].is_string());
    }
    assert_eq!(outcomes[0]["usage"], usage_json(FIRST_USAGE));
    assert_eq!(outcomes[1]["usage"], usage_json(SECOND_USAGE));
}

fn assert_archive_contains_no_private_canaries(bytes: &[u8]) {
    for canary in [
        USER_CANARY,
        TOOL_CANARY,
        WARM_RESPONSE_ID,
        FIRST_RESPONSE_ID,
        SECOND_RESPONSE_ID,
    ] {
        assert!(
            !bytes
                .windows(canary.len())
                .any(|window| window == canary.as_bytes()),
            "diagnostic archive exposed private canary {canary}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_diagnostics_link_each_request_to_its_exact_usage_without_plaintext() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let response_mock = mount_sse_sequence(
        &server,
        two_response_tool_sequence().into_iter().map(sse).collect(),
    )
    .await;
    let test = test_codex().build_with_auto_env(&server).await?;

    test.submit_turn(USER_CANARY).await?;
    test.codex.shutdown_and_wait().await?;

    let outbound = response_mock.requests();
    assert_eq!(outbound.len(), 2);
    let wire_sizes = [
        outbound[0].body_bytes().len(),
        outbound[1].body_bytes().len(),
    ];
    let logical_sizes = [
        serde_json::to_vec(&outbound[0].body_json())?.len(),
        serde_json::to_vec(&outbound[1].body_json())?.len(),
    ];
    let archive = read_diagnostic_archive(test.home.path());
    assert_two_linked_attempts(
        &archive.records,
        /*first_sequence*/ 1,
        "http",
        &wire_sizes,
        Some(&logical_sizes),
        &[0, 0],
    );
    let diagnostic_ids = [
        archive.records[0]["runId"]
            .as_str()
            .expect("diagnostic run id should be a string"),
        archive.records[0]["attemptId"]
            .as_str()
            .expect("first diagnostic attempt id should be a string"),
        archive.records[2]["attemptId"]
            .as_str()
            .expect("second diagnostic attempt id should be a string"),
    ];
    for request in &outbound {
        let body = request.body_bytes();
        for private_marker in
            diagnostic_ids
                .into_iter()
                .chain(["schemaVersion", "requestKind", "retryOrdinal"])
        {
            assert!(
                !body
                    .windows(private_marker.len())
                    .any(|window| window == private_marker.as_bytes()),
                "diagnostic-only marker entered a model request: {private_marker}"
            );
        }
    }
    assert_archive_contains_no_private_canaries(&archive.bytes);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reused_websocket_diagnostics_do_not_keep_the_first_attempt_observer() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let mut scripted_requests = vec![vec![
        ev_response_created(WARM_RESPONSE_ID),
        ev_completed(WARM_RESPONSE_ID),
    ]];
    scripted_requests.extend(two_response_tool_sequence());
    let server = start_websocket_server(vec![scripted_requests]).await;
    let mut builder = test_codex();
    let test = builder.build_with_websocket_server(&server).await?;

    test.submit_turn(USER_CANARY).await?;
    test.codex.shutdown_and_wait().await?;

    assert_eq!(server.handshakes().len(), 1);
    let outbound = server.single_connection();
    assert_eq!(outbound.len(), 3);
    let wire_sizes = [
        outbound[1].body_json().to_string().len(),
        outbound[2].body_json().to_string().len(),
    ];
    let archive = read_diagnostic_archive(test.home.path());
    assert_eq!(archive.records.len(), 6);
    let warmup_request = &archive.records[0];
    let warmup_outcome = &archive.records[1];
    assert_eq!(warmup_request["sequence"], 1);
    assert_eq!(warmup_request["event"], "request");
    assert_eq!(warmup_request["requestKind"], "warmup");
    assert_eq!(warmup_request["retryOrdinal"], 0);
    assert_eq!(warmup_request["wire"]["kind"], "websocket");
    assert_eq!(
        warmup_request["wire"]["body"]["bytes"],
        outbound[0].body_json().to_string().len()
    );
    assert_eq!(warmup_request["wire"]["transport"]["endpoint"], "responses");
    assert_eq!(
        warmup_request["wire"]["transport"]["connectionReused"],
        false
    );
    assert_eq!(warmup_request["wire"]["transport"]["incremental"], false);
    assert_eq!(warmup_request["attemptId"], warmup_outcome["attemptId"]);
    assert_eq!(warmup_outcome["sequence"], 2);
    assert_eq!(warmup_outcome["event"], "outcome");
    assert_eq!(warmup_outcome["terminal"], "completed");
    assert_eq!(warmup_outcome["usage"], usage_json(ZERO_USAGE));
    assert_two_linked_attempts(
        &archive.records[2..],
        /*first_sequence*/ 3,
        "websocket",
        &wire_sizes,
        None,
        &[0, 0],
    );
    assert_ne!(warmup_request["attemptId"], archive.records[2]["attemptId"]);
    assert_ne!(warmup_request["attemptId"], archive.records[4]["attemptId"]);
    assert_archive_contains_no_private_canaries(&archive.bytes);

    server.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unavailable_diagnostic_directory_does_not_break_or_modify_inference() -> Result<()> {
    skip_if_no_network!(Ok(()));

    const BLOCKER_CANARY: &str = "PRIVATE-DIAGNOSTIC-OPEN-FAILURE";
    const COLLECTION_ERROR: &str = "cache diagnostic collection is unavailable";

    let server = start_mock_server().await;
    let response_mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("response-after-diagnostic-open-failure"),
            ev_assistant_message("message-after-diagnostic-open-failure", "done"),
            ev_completed("response-after-diagnostic-open-failure"),
        ]),
    )
    .await;
    let home = Arc::new(TempDir::new()?);
    fs::write(home.path().join("cache-diagnostics"), BLOCKER_CANARY)?;
    let test = test_codex()
        .with_home(Arc::clone(&home))
        .build_with_auto_env(&server)
        .await?;

    test.submit_turn("continue despite unavailable diagnostics")
        .await?;
    test.codex.shutdown_and_wait().await?;

    let request = response_mock.single_request();
    assert!(request.body_contains_text("continue despite unavailable diagnostics"));
    assert!(!request.body_contains_text(BLOCKER_CANARY));
    assert!(!request.body_contains_text(COLLECTION_ERROR));
    assert_eq!(
        fs::read(home.path().join("cache-diagnostics"))?,
        BLOCKER_CANARY.as_bytes()
    );

    Ok(())
}

#[path = "cache_diagnostics_lifecycle.rs"]
mod lifecycle;
