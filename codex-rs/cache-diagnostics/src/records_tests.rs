use super::*;
use codex_api::ResponsesApiRequest;
use codex_api::ResponsesApiTools;
use codex_protocol::models::ResponseItem;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use serde_json::value::RawValue;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::sync::Barrier;
use std::thread;
use tempfile::TempDir;

const PRIVATE_CANARY: &str = "PRIVATE-CACHE-DIAGNOSTIC-CANARY";
const TRANSPORT_PRIVATE_CANARY: &[u8] = b"PRIVATE-TRANSPORT-IDENTITY-CANARY";
const MAX_RECORD_BYTES: usize = 256 * 1024;

fn request(canary: &str, input_count: usize, tool_count: usize) -> ResponsesApiRequest {
    let input = (0..input_count)
        .map(|index| {
            serde_json::from_value::<ResponseItem>(json!({
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_text",
                    "text": format!("prompt-{canary}-{index}"),
                }],
            }))
            .expect("fixture response item should deserialize")
        })
        .collect();
    // Tool names are the one deliberate plaintext field in the archive (they
    // already appear in rollout function_call records), so the fixtures keep
    // them canary-free while every other tool field still carries the canary.
    let tools = (0..tool_count)
        .map(|index| {
            json!({
                "type": "function",
                "name": format!("tool-name-{index}"),
                "description": format!("description-{canary}-{index}"),
                "parameters": {
                    "type": "object",
                    "properties": {
                        format!("dynamic-key-{canary}-{index}"): {"type": "string"}
                    },
                },
            })
        })
        .collect::<Vec<_>>();
    let raw_tools = RawValue::from_string(
        serde_json::to_string(&tools).expect("fixture tools should serialize"),
    )
    .expect("fixture tools should be valid raw JSON");

    ResponsesApiRequest {
        model: format!("model-{canary}"),
        instructions: format!("instructions-{canary}"),
        input,
        tools: Some(ResponsesApiTools::from(Arc::<RawValue>::from(raw_tools))),
        tool_choice: format!("choice-{canary}"),
        parallel_tool_calls: true,
        reasoning: None,
        store: false,
        stream: true,
        stream_options: None,
        include: vec![format!("include-{canary}")],
        service_tier: Some(format!("tier-{canary}")),
        prompt_cache_key: Some(format!("prompt-cache-key-{canary}")),
        text: None,
        client_metadata: Some(HashMap::from([(
            format!("metadata-key-{canary}"),
            format!("metadata-value-{canary}"),
        )])),
        access_programs: None,
    }
}

fn context(canary: &str) -> AttemptContext<'_> {
    AttemptContext {
        request_kind: RequestKind::Turn,
        retry_ordinal: 0,
        thread_id: Some(canary),
        session_id: Some(canary),
        turn_id: Some(canary),
        parent_id: Some(canary),
        affinity_id: Some(canary),
        previous_response_id: Some(canary),
    }
}

fn transport_identity(value: &[u8]) -> TransportIdentity<'_> {
    TransportIdentity {
        session_id: Some(value),
        thread_id: Some(value),
        client_request_id: Some(value),
        subagent: Some(value),
        routing_hint: Some(value),
        responses_lite: Some(value),
        previous_response_id: Some(value),
        originator: Some(value),
        user_agent: Some(value),
    }
}

fn usage() -> TokenUsage {
    TokenUsage {
        input: 101,
        cached_input: 79,
        cache_write: 11,
        output: 23,
        reasoning: 7,
        total: 131,
    }
}

fn observe_and_complete(collector: &Arc<Collector>, canary: &str, wire: &[u8]) {
    let logical = request(canary, 2, 2);
    let attempt = collector.start_attempt(context(canary), &logical);
    attempt.observe_wire_request(WireRequest {
        kind: WireRequestKind::Http,
        body: wire,
    });
    attempt.completed(CompletedOutcome {
        response_id: Some(canary),
        usage: usage(),
    });
}

fn jsonl_paths(root: &Path) -> Vec<std::path::PathBuf> {
    fn visit(path: &Path, files: &mut Vec<std::path::PathBuf>) {
        for entry in fs::read_dir(path).expect("diagnostic directory should be readable") {
            let path = entry.expect("directory entry should be readable").path();
            if path.is_dir() {
                visit(&path, files);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("jsonl") {
                files.push(path);
            }
        }
    }

    let mut files = Vec::new();
    visit(root, &mut files);
    files.sort();
    files
}

fn record_lines(root: &Path) -> Vec<Vec<u8>> {
    jsonl_paths(root)
        .into_iter()
        .flat_map(|path| {
            BufReader::new(fs::File::open(path).expect("record file should open"))
                .split(b'\n')
                .map(|line| line.expect("record line should be readable"))
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn records(root: &Path) -> Vec<Value> {
    record_lines(root)
        .iter()
        .map(|line| serde_json::from_slice(line).expect("record line should be valid JSON"))
        .collect()
}

fn request_records(root: &Path) -> Vec<Value> {
    records(root)
        .into_iter()
        .filter(|record| record["event"] == "request")
        .collect()
}

fn hmacs(value: &Value) -> Vec<String> {
    fn visit(value: &Value, output: &mut Vec<String>) {
        match value {
            Value::Object(object) => {
                for (key, value) in object {
                    if key == "hmac" {
                        output.push(
                            value
                                .as_str()
                                .expect("an hmac field should contain a string")
                                .to_owned(),
                        );
                    } else {
                        visit(value, output);
                    }
                }
            }
            Value::Array(values) => {
                for value in values {
                    visit(value, output);
                }
            }
            _ => {}
        }
    }

    let mut output = Vec::new();
    visit(value, &mut output);
    output
}

fn find_request<'a>(records: &'a [Value], attempt_id: &Value) -> &'a Value {
    records
        .iter()
        .find(|record| record["event"] == "request" && record["attemptId"] == *attempt_id)
        .expect("attempt should have a request record")
}

#[test]
fn record_file_contains_no_sensitive_plaintext_and_links_outcome() {
    let home = TempDir::new().expect("temporary home should be created");
    let first_date = chrono::Utc::now().format("%Y/%m/%d").to_string();
    let collector = Collector::open(home.path()).expect("collector should open");
    let last_date = chrono::Utc::now().format("%Y/%m/%d").to_string();

    observe_and_complete(
        &collector,
        PRIVATE_CANARY,
        br#"{"wire":"PRIVATE-CACHE-DIAGNOSTIC-CANARY"}"#,
    );

    let paths = jsonl_paths(home.path());
    assert_eq!(paths.len(), 1);
    let directory = paths[0].parent().expect("run directory");
    let archive = home.path().join("cache-diagnostics");
    assert!(directory == archive.join(first_date) || directory == archive.join(last_date));
    assert!(!paths[0].to_string_lossy().contains(PRIVATE_CANARY));
    let lines = record_lines(home.path());
    assert_eq!(lines.len(), 2);
    for line in &lines {
        assert!(line.len() < MAX_RECORD_BYTES);
        assert!(
            !line
                .windows(PRIVATE_CANARY.len())
                .any(|window| window == PRIVATE_CANARY.as_bytes())
        );
    }

    let records = records(home.path());
    let request = records
        .iter()
        .find(|record| record["event"] == "request")
        .expect("request record should exist");
    let outcome = records
        .iter()
        .find(|record| record["event"] == "outcome")
        .expect("outcome record should exist");
    assert_eq!(request["attemptId"], outcome["attemptId"]);
    assert_eq!(outcome["terminal"], "completed");
    assert_eq!(outcome["usage"], serde_json::to_value(usage()).unwrap());
    assert!(outcome["responseId"]["hmac"].is_string());
}

#[test]
fn continuation_and_tool_provenance_persist_on_the_request_record() {
    let home = TempDir::new().unwrap();
    let collector = Collector::open(home.path()).unwrap();
    let logical = request(PRIVATE_CANARY, 2, 2);

    let attempt = collector.start_attempt(context(PRIVATE_CANARY), &logical);
    attempt.record_continuation(ContinuationReport {
        transport: ContinuationTransport::Websocket,
        decision: ContinuationDecision::Full,
        drop_reason: Some(ContinuationDropReason::InputPrefixMismatch),
        first_mismatched_property: None,
        first_mismatched_input_index: Some(3),
        divergence: Some(ContinuationDivergence {
            scope: DivergenceScope::InputItem,
            byte_offset: 17,
            previous_bytes: 120,
            current_bytes: 114,
        }),
    });
    attempt.record_tool_provenance(ToolProvenance {
        model_preset_count: 4,
        model_catalog_lock_contention_fallback: true,
        model_catalog_identity: Some(format!("catalog-{PRIVATE_CANARY}").as_bytes()),
        role_file_read_failures: 1,
    });
    attempt.observe_wire_request(WireRequest {
        kind: WireRequestKind::Http,
        body: b"wire",
    });
    attempt.completed(CompletedOutcome {
        response_id: Some(PRIVATE_CANARY),
        usage: usage(),
    });

    let requests = request_records(home.path());
    assert_eq!(requests.len(), 1);
    let record = &requests[0];
    assert_eq!(record["schemaVersion"], 2);
    assert_eq!(
        record["continuation"],
        json!({
            "transport": "websocket",
            "decision": "full",
            "dropReason": "inputPrefixMismatch",
            "firstMismatchedInputIndex": 3,
            "divergence": {
                "scope": "inputItem",
                "byteOffset": 17,
                "previousBytes": 120,
                "currentBytes": 114,
            },
        })
    );
    let provenance = &record["toolProvenance"];
    assert_eq!(provenance["modelCatalog"]["presetCount"], 4);
    assert_eq!(provenance["modelCatalog"]["lockContentionFallback"], true);
    assert!(provenance["modelCatalog"]["identity"]["hmac"].is_string());
    assert_eq!(provenance["roleFileReadFailures"], 1);
    // The raw catalog identity must be fingerprinted, never stored.
    for line in record_lines(home.path()) {
        assert!(
            !line
                .windows(PRIVATE_CANARY.len())
                .any(|window| window == PRIVATE_CANARY.as_bytes())
        );
    }

    let detail = &record["logical"]["toolsDetail"];
    assert_eq!(detail["count"], 2);
    assert_eq!(detail["retained"][0]["name"], "tool-name-0");
    assert_eq!(detail["retained"][1]["name"], "tool-name-1");
    assert!(detail["retained"][0]["description"]["hmac"].is_string());
    assert!(detail["retained"][0]["parameters"]["hmac"].is_string());
    assert_ne!(
        detail["retained"][0]["description"]["hmac"],
        detail["retained"][1]["description"]["hmac"]
    );
}

#[test]
fn continuation_and_provenance_after_persistence_are_ignored() {
    let home = TempDir::new().unwrap();
    let collector = Collector::open(home.path()).unwrap();
    let logical = request("late", 1, 1);

    let attempt = collector.start_attempt(context("late"), &logical);
    attempt.observe_wire_request(WireRequest {
        kind: WireRequestKind::Http,
        body: b"wire",
    });
    attempt.record_continuation(ContinuationReport {
        transport: ContinuationTransport::Websocket,
        decision: ContinuationDecision::Incremental,
        drop_reason: None,
        first_mismatched_property: None,
        first_mismatched_input_index: None,
        divergence: None,
    });
    attempt.terminal(TerminalOutcome::Failed);

    let requests = request_records(home.path());
    assert_eq!(requests.len(), 1);
    assert!(requests[0].get("continuation").is_none());
}

#[test]
fn persisted_key_makes_fingerprints_stable_and_installations_isolated() {
    let home_a = TempDir::new().unwrap();
    let home_b = TempDir::new().unwrap();
    let home_c = TempDir::new().unwrap();
    let collector_a = Collector::open(home_a.path()).unwrap();
    let key_a = home_a.path().join("cache-diagnostics/.fingerprint-key");
    let key_b = home_b.path().join("cache-diagnostics/.fingerprint-key");
    fs::create_dir_all(key_b.parent().unwrap()).unwrap();
    fs::copy(&key_a, &key_b).unwrap();
    let collector_b = Collector::open(home_b.path()).unwrap();
    let collector_c = Collector::open(home_c.path()).unwrap();

    for collector in [&collector_a, &collector_b, &collector_c] {
        observe_and_complete(collector, PRIVATE_CANARY, b"same exact wire body");
    }

    let a = hmacs(&request_records(home_a.path())[0]);
    let b = hmacs(&request_records(home_b.path())[0]);
    let c = hmacs(&request_records(home_c.path())[0]);
    assert!(!a.is_empty());
    assert_eq!(a, b);
    assert_ne!(a, c);
    let scope_a = request_records(home_a.path())[0]["keyScope"].clone();
    let scope_b = request_records(home_b.path())[0]["keyScope"].clone();
    let scope_c = request_records(home_c.path())[0]["keyScope"].clone();
    assert!(
        scope_a["hmac"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert_eq!(scope_a, scope_b);
    assert_ne!(scope_a, scope_c);
    assert_eq!(fs::read(key_a).unwrap().len(), 32);
    assert_eq!(fs::read(key_b).unwrap().len(), 32);
}

#[cfg(unix)]
#[test]
fn installation_key_is_created_private_and_concurrently_once() {
    use std::os::unix::fs::PermissionsExt;

    let home = Arc::new(TempDir::new().unwrap());
    let barrier = Arc::new(Barrier::new(16));
    let threads = (0..16)
        .map(|_| {
            let home = Arc::clone(&home);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                Collector::open(home.path())
            })
        })
        .collect::<Vec<_>>();
    for thread in threads {
        thread
            .join()
            .expect("open thread should not panic")
            .unwrap();
    }

    let key = home.path().join("cache-diagnostics/.fingerprint-key");
    assert_eq!(fs::read(&key).unwrap().len(), 32);
    assert_eq!(
        fs::metadata(key).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn malformed_key_is_not_replaced_and_error_leaks_no_contents_or_path() {
    let home = TempDir::new().unwrap();
    let key = home.path().join("cache-diagnostics/.fingerprint-key");
    fs::create_dir_all(key.parent().unwrap()).unwrap();
    let malformed = format!("malformed-{PRIVATE_CANARY}");
    fs::write(&key, malformed.as_bytes()).unwrap();

    let error = match Collector::open(home.path()) {
        Ok(_) => panic!("malformed key should disable collection"),
        Err(error) => error,
    };

    assert_eq!(fs::read(&key).unwrap(), malformed.as_bytes());
    assert!(jsonl_paths(home.path()).is_empty());
    let rendered = format!("{error:?} {error}");
    assert!(!rendered.contains(PRIVATE_CANARY));
    assert!(!rendered.contains(&home.path().to_string_lossy().to_string()));
}

#[test]
fn wire_fingerprint_hashes_exact_bytes_and_missing_observation_is_truthful() {
    let home = TempDir::new().unwrap();
    let collector = Collector::open(home.path()).unwrap();
    let logical = request("logical", 1, 1);

    let compact = collector.start_attempt(context("identity"), &logical);
    compact.observe_wire_request(WireRequest {
        kind: WireRequestKind::Http,
        body: br#"{"a":1,"b":2}"#,
    });
    compact.terminal(TerminalOutcome::Failed);

    let spaced = collector.start_attempt(context("identity"), &logical);
    spaced.observe_wire_request(WireRequest {
        kind: WireRequestKind::Http,
        body: br#"{ "a": 1, "b": 2 }"#,
    });
    spaced.terminal(TerminalOutcome::Failed);

    let missing = collector.start_attempt(context("identity"), &logical);
    missing.terminal(TerminalOutcome::Cancelled);

    let requests = request_records(home.path());
    assert_eq!(requests.len(), 3);
    assert_eq!(
        requests[0]["logical"]["body"],
        requests[1]["logical"]["body"]
    );
    assert_ne!(requests[0]["wire"]["body"], requests[1]["wire"]["body"]);
    assert_eq!(requests[0]["wire"]["body"]["bytes"], 13);
    assert_eq!(requests[1]["wire"]["body"]["bytes"], 18);
    assert_eq!(requests[2]["wire"], json!({"status": "missing"}));
}

#[test]
fn transport_records_preserve_closed_facts_and_domain_separate_identity_hmacs() {
    let home = TempDir::new().unwrap();
    let collector = Collector::open(home.path()).unwrap();
    let logical = request("logical", 1, 1);

    let http = collector.start_attempt(context("http"), &logical);
    http.observe_transport_request(TransportRequest {
        body: br#"{"transport":"http"}"#,
        transport: TransportMetadata::Http {
            endpoint: EndpointKind::Guardian,
            compression: CompressionKind::Zstd,
            identity: transport_identity(TRANSPORT_PRIVATE_CANARY),
        },
    });
    http.terminal(TerminalOutcome::Failed);

    let websocket = collector.start_attempt(context("websocket"), &logical);
    websocket.observe_transport_request(TransportRequest {
        body: br#"{"transport":"websocket"}"#,
        transport: TransportMetadata::Websocket {
            endpoint: EndpointKind::GuardianClassifier,
            connection_reused: false,
            incremental: true,
            identity: transport_identity(TRANSPORT_PRIVATE_CANARY),
        },
    });
    websocket.terminal(TerminalOutcome::Fallback);

    let lines = record_lines(home.path());
    assert_eq!(lines.len(), 4);
    for line in &lines {
        assert!(line.len() < MAX_RECORD_BYTES);
        assert!(
            !line
                .windows(TRANSPORT_PRIVATE_CANARY.len())
                .any(|window| window == TRANSPORT_PRIVATE_CANARY)
        );
    }

    let requests = request_records(home.path());
    assert_eq!(requests.len(), 2);
    let http_wire = &requests[0]["wire"];
    assert_eq!(http_wire["kind"], "http");
    assert_eq!(http_wire["body"]["bytes"], 20);
    let websocket_wire = &requests[1]["wire"];
    assert_eq!(websocket_wire["kind"], "websocket");
    assert_eq!(websocket_wire["body"]["bytes"], 25);

    let mut http_facts = http_wire["transport"].as_object().unwrap().clone();
    let http_identity = http_facts.remove("identity").unwrap();
    assert_eq!(
        Value::Object(http_facts),
        json!({
            "endpoint": "guardian",
            "compression": "zstd",
            "connectionReused": {"status": "unavailable"},
            "incremental": {"status": "unavailable"},
        })
    );
    let mut websocket_facts = websocket_wire["transport"].as_object().unwrap().clone();
    let websocket_identity = websocket_facts.remove("identity").unwrap();
    assert_eq!(
        Value::Object(websocket_facts),
        json!({
            "endpoint": "guardianClassifier",
            "compression": {"status": "unavailable"},
            "connectionReused": false,
            "incremental": true,
        })
    );
    assert_eq!(http_identity, websocket_identity);

    let identity = http_identity.as_object().unwrap();
    let mut fields = identity.keys().cloned().collect::<Vec<_>>();
    fields.sort();
    let expected_fields = [
        "clientRequestId",
        "originator",
        "previousResponseId",
        "responsesLite",
        "routingHint",
        "sessionId",
        "subagent",
        "threadId",
        "userAgent",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    assert_eq!(fields, expected_fields);
    let hmacs = identity
        .values()
        .map(|fingerprint| {
            assert_eq!(fingerprint["bytes"], TRANSPORT_PRIVATE_CANARY.len());
            let hmac = fingerprint["hmac"].as_str().unwrap();
            assert!(!hmac.is_empty());
            hmac
        })
        .collect::<HashSet<_>>();
    assert_eq!(hmacs.len(), 9);
}

#[test]
fn bounded_manifests_hash_the_complete_omitted_tail() {
    let home = TempDir::new().unwrap();
    let collector = Collector::open(home.path()).unwrap();
    let first = request(PRIVATE_CANARY, 10_000, 10_000);
    let mut second = first.clone();
    let last = second.input.last_mut().unwrap();
    *last = serde_json::from_value(json!({
        "type": "message",
        "role": "user",
        "content": [{"type": "input_text", "text": "different omitted tail"}],
    }))
    .unwrap();

    for logical in [&first, &second] {
        let attempt = collector.start_attempt(context(PRIVATE_CANARY), logical);
        attempt.observe_transport_request(TransportRequest {
            body: PRIVATE_CANARY.as_bytes(),
            transport: TransportMetadata::Http {
                endpoint: EndpointKind::Responses,
                compression: CompressionKind::None,
                identity: transport_identity(TRANSPORT_PRIVATE_CANARY),
            },
        });
        attempt.terminal(TerminalOutcome::Cancelled);
    }

    let lines = record_lines(home.path());
    assert_eq!(lines.len(), 4);
    assert!(lines.iter().all(|line| line.len() < MAX_RECORD_BYTES));
    assert!(lines.iter().all(|line| {
        !line
            .windows(PRIVATE_CANARY.len())
            .any(|window| window == PRIVATE_CANARY.as_bytes())
    }));
    assert!(lines.iter().all(|line| {
        !line
            .windows(TRANSPORT_PRIVATE_CANARY.len())
            .any(|window| window == TRANSPORT_PRIVATE_CANARY)
    }));
    let requests = request_records(home.path());
    assert_eq!(requests.len(), 2);
    let first_input = &requests[0]["logical"]["input"];
    let second_input = &requests[1]["logical"]["input"];
    let retained = first_input["retained"].as_array().unwrap().len();
    assert!(retained < 10_000);
    assert_eq!(first_input["count"], 10_000);
    assert_eq!(first_input["omitted"]["count"], 10_000 - retained);
    assert_ne!(
        first_input["omitted"]["hmac"],
        second_input["omitted"]["hmac"]
    );
    let tools = &requests[0]["logical"]["tools"];
    let retained_tools = tools["retained"].as_array().unwrap().len();
    assert!(retained_tools < 10_000);
    assert_eq!(tools["omitted"]["count"], 10_000 - retained_tools);
}

#[test]
fn duplicate_terminal_notification_writes_one_outcome() {
    let home = TempDir::new().unwrap();
    let collector = Collector::open(home.path()).unwrap();
    let logical = request("logical", 1, 1);
    let attempt = collector.start_attempt(context("identity"), &logical);
    attempt.terminal(TerminalOutcome::Failed);
    attempt.terminal(TerminalOutcome::Cancelled);

    let records = records(home.path());
    let outcome = records
        .iter()
        .find(|record| record["event"] == "outcome")
        .unwrap();
    assert_eq!(
        records
            .iter()
            .filter(|record| record["event"] == "request")
            .count(),
        1
    );
    assert_eq!(
        records
            .iter()
            .filter(|record| record["event"] == "outcome")
            .count(),
        1
    );
    assert_eq!(
        find_request(&records, &outcome["attemptId"])["wire"],
        json!({"status": "missing"})
    );
}

#[test]
fn completed_outcomes_preserve_negative_usage_and_keep_absent_usage_null() {
    let home = TempDir::new().unwrap();
    let collector = Collector::open(home.path()).unwrap();
    let logical = request("logical", 1, 1);

    let signed = collector.start_attempt(context("signed"), &logical);
    signed.completed(CompletedOutcome {
        response_id: Some("signed-response"),
        usage: TokenUsage {
            input: -1,
            cached_input: -2,
            cache_write: -3,
            output: -4,
            reasoning: -5,
            total: -6,
        },
    });

    let absent = collector.start_attempt(context("absent"), &logical);
    absent.completed_without_usage(Some("absent-response"));

    let records = records(home.path());
    let outcomes = records
        .iter()
        .filter(|record| record["event"] == "outcome")
        .collect::<Vec<_>>();
    assert_eq!(outcomes.len(), 2);
    assert_eq!(outcomes[0]["terminal"], "completed");
    assert_eq!(
        outcomes[0]["usage"],
        json!({
            "input": -1,
            "cachedInput": -2,
            "cacheWrite": -3,
            "output": -4,
            "reasoning": -5,
            "total": -6,
        })
    );
    assert_eq!(outcomes[1]["terminal"], "completed");
    assert_eq!(outcomes[1]["usage"], Value::Null);
    assert_eq!(outcomes[1]["responseId"]["bytes"], 15);
    assert!(
        outcomes[1]["responseId"]["hmac"]
            .as_str()
            .is_some_and(|hmac| !hmac.is_empty())
    );
    assert_eq!(
        find_request(&records, &outcomes[1]["attemptId"])["wire"],
        json!({"status": "missing"})
    );
}
