use std::collections::HashMap;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use codex_api::ResponsesApiRequest;
use codex_api::ResponsesApiTools;
use codex_cache_diagnostics::AttemptContext;
use codex_cache_diagnostics::Collector;
use codex_cache_diagnostics::ContinuationDecision;
use codex_cache_diagnostics::ContinuationDivergence;
use codex_cache_diagnostics::ContinuationDropReason;
use codex_cache_diagnostics::ContinuationReport;
use codex_cache_diagnostics::ContinuationTransport;
use codex_cache_diagnostics::DivergenceScope;
use codex_cache_diagnostics::RequestKind;
use codex_cache_diagnostics::TerminalOutcome;
use codex_cache_diagnostics::ToolProvenance;
use codex_cache_diagnostics::WireRequest;
use codex_cache_diagnostics::WireRequestKind;
use codex_protocol::models::ResponseItem;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use serde_json::value::RawValue;
use tempfile::TempDir;

const PRIVATE_CANARY: &str = "PRIVATE-CACHE-DIAGNOSTIC-CANARY";
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
    let tools = (0..tool_count)
        .map(|index| {
            json!({
                "type": "function",
                "name": format!("public-tool-{index}"),
                "description": format!("description-{canary}-{index}"),
                "parameters": {
                    "type": "object",
                    "properties": {
                        format!("private-key-{canary}-{index}"): {"type": "string"}
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

fn context(identity: &str) -> AttemptContext<'_> {
    AttemptContext {
        request_kind: RequestKind::Turn,
        retry_ordinal: 0,
        thread_id: Some(identity),
        session_id: Some(identity),
        turn_id: Some(identity),
        parent_id: Some(identity),
        affinity_id: Some(identity),
        previous_response_id: Some(identity),
    }
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

fn record_lines(path: &Path) -> Vec<Vec<u8>> {
    BufReader::new(fs::File::open(path).expect("record file should open"))
        .split(b'\n')
        .map(|line| line.expect("record line should be readable"))
        .filter(|line| !line.is_empty())
        .collect()
}

fn records(path: &Path) -> Vec<Value> {
    record_lines(path)
        .iter()
        .map(|line| serde_json::from_slice(line).expect("record line should be valid JSON"))
        .collect()
}

fn request_records(path: &Path) -> Vec<Value> {
    records(path)
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

fn terminal_attempt(collector: &Arc<Collector>, identity: &str, wire: Option<&[u8]>) {
    let logical = request(PRIVATE_CANARY, 1, 1);
    let attempt = collector.start_attempt(context(identity), &logical);
    if let Some(wire) = wire {
        attempt.observe_wire_request(WireRequest {
            kind: WireRequestKind::Http,
            body: wire,
        });
    }
    attempt.terminal(TerminalOutcome::Failed);
}

#[test]
fn installation_key_is_private_stable_and_scoped_to_one_installation() {
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
        terminal_attempt(collector, "same-identity", Some(b"same exact wire body"));
    }

    let a = request_records(&jsonl_paths(home_a.path())[0]);
    let b = request_records(&jsonl_paths(home_b.path())[0]);
    let c = request_records(&jsonl_paths(home_c.path())[0]);
    assert_eq!(a.len(), 1);
    assert_eq!(hmacs(&a[0]), hmacs(&b[0]));
    assert_ne!(hmacs(&a[0]), hmacs(&c[0]));
    assert_eq!(a[0]["keyScope"], b[0]["keyScope"]);
    assert_ne!(a[0]["keyScope"], c[0]["keyScope"]);
    assert_eq!(fs::read(&key_a).unwrap().len(), 32);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        assert_eq!(
            fs::metadata(key_a).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn run_file_only_appends_complete_json_lines_and_preserves_old_evidence() {
    let home = TempDir::new().unwrap();
    let old_directory = home.path().join("cache-diagnostics/2000/01/01");
    let old_path = old_directory.join("run-preserve.jsonl");
    fs::create_dir_all(&old_directory).unwrap();
    let old_line = serde_json::to_string(&json!({
        "schemaVersion": 1,
        "event": "request",
        "padding": "x".repeat(1024),
    }))
    .unwrap();
    let old_evidence = format!("{old_line}\n").repeat(1024);
    fs::write(&old_path, &old_evidence).unwrap();

    let collector = Collector::open(home.path()).unwrap();
    terminal_attempt(&collector, "first", Some(b"first"));
    let run_path = jsonl_paths(home.path())
        .into_iter()
        .find(|path| path != &old_path)
        .expect("new collector run file should exist");
    let first_append = fs::read(&run_path).unwrap();

    terminal_attempt(&collector, "second", Some(b"second"));
    let second_append = fs::read(&run_path).unwrap();
    assert!(second_append.starts_with(&first_append));
    assert!(second_append.len() > first_append.len());
    assert_eq!(
        records(&run_path)
            .iter()
            .map(|record| record["sequence"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    assert_eq!(fs::read_to_string(old_path).unwrap(), old_evidence);
}

#[test]
fn wire_fingerprint_uses_exact_bytes_and_missing_is_explicit() {
    let home = TempDir::new().unwrap();
    let collector = Collector::open(home.path()).unwrap();
    terminal_attempt(&collector, "compact", Some(br#"{"a":1,"b":2}"#));
    terminal_attempt(&collector, "spaced", Some(br#"{ "a": 1, "b": 2 }"#));
    terminal_attempt(&collector, "missing", None);

    let requests = request_records(&jsonl_paths(home.path())[0]);
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
fn first_terminal_notification_wins_and_writes_one_linked_outcome() {
    let home = TempDir::new().unwrap();
    let collector = Collector::open(home.path()).unwrap();
    let logical = request(PRIVATE_CANARY, 1, 1);
    let attempt = collector.start_attempt(context("identity"), &logical);

    attempt.terminal(TerminalOutcome::Failed);
    attempt.terminal(TerminalOutcome::Cancelled);

    let records = records(&jsonl_paths(home.path())[0]);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["event"], "request");
    assert_eq!(records[1]["event"], "outcome");
    assert_eq!(records[1]["terminal"], "failed");
    assert_eq!(records[0]["attemptId"], records[1]["attemptId"]);
}

#[test]
fn maximal_request_stays_bounded_and_keeps_safe_tool_decision_evidence() {
    let home = TempDir::new().unwrap();
    let collector = Collector::open(home.path()).unwrap();
    let logical = request(PRIVATE_CANARY, 10_000, 10_000);
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
        body: PRIVATE_CANARY.as_bytes(),
    });
    attempt.terminal(TerminalOutcome::Cancelled);

    let run_path = &jsonl_paths(home.path())[0];
    let lines = record_lines(run_path);
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|line| line.len() < MAX_RECORD_BYTES));
    assert!(lines.iter().all(|line| {
        !line
            .windows(PRIVATE_CANARY.len())
            .any(|window| window == PRIVATE_CANARY.as_bytes())
    }));

    let record = &request_records(run_path)[0];
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
    assert_eq!(record["toolProvenance"]["modelCatalog"]["presetCount"], 4);
    assert_eq!(
        record["toolProvenance"]["modelCatalog"]["lockContentionFallback"],
        true
    );
    assert!(record["toolProvenance"]["modelCatalog"]["identity"]["hmac"].is_string());
    assert_eq!(record["toolProvenance"]["roleFileReadFailures"], 1);

    let details = &record["logical"]["toolsDetail"];
    let retained = details["retained"].as_array().unwrap();
    assert_eq!(details["count"], 10_000);
    assert!(retained.len() < 10_000);
    assert_eq!(details["omitted"]["count"], 10_000 - retained.len());
    assert_eq!(retained[0]["name"], "public-tool-0");
    assert!(retained[0]["descriptor"]["bytes"].is_u64());
    assert!(retained[0]["descriptor"]["hmac"].is_string());
    assert!(retained[0]["description"]["bytes"].is_u64());
    assert!(retained[0]["description"]["hmac"].is_string());
    assert!(retained[0]["parameters"]["bytes"].is_u64());
    assert!(retained[0]["parameters"]["hmac"].is_string());
}
