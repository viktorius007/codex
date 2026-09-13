use std::time::Duration;

use anyhow::Result;
use anyhow::anyhow;
use codex_features::Feature;
use codex_login::CodexAuth;
use codex_protocol::openai_models::ReasoningEffort;
use core_test_support::responses::ResponsesRequest;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call_with_namespace;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_once_match;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::responses::strip_metadata_from_json;
use core_test_support::responses::strip_response_item_ids_from_json;
use core_test_support::test_codex::test_codex;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

const MODEL: &str = "gpt-5.2";
const BASE_INSTRUCTIONS: &str = "stable base instructions for the cache-prefix test";
const DEVELOPER_INSTRUCTIONS: &str = "stable developer instructions for the cache-prefix test";
const GLOBAL_INSTRUCTIONS: &str = "stable global instructions for the cache-prefix test";
const ROOT_ROLE_HINT: &str = "root-only cache-prefix boundary";
const CHILD_ROLE_HINT: &str = "child-only cache-prefix boundary";
const ROOT_PROMPT: &str = "private parent task: delegate the cache audit";
const CHILD_TASK: &str = "private child task: inspect the repository";
const SPAWN_CALL_ID: &str = "spawn-worker";
const COLLABORATION_NAMESPACE: &str = "collaboration";

fn body_contains(request: &wiremock::Request, text: &str) -> bool {
    serde_json::from_slice::<Value>(&request.body).is_ok_and(|body| body.to_string().contains(text))
}

fn request_has_input_type(request: &wiremock::Request, input_type: &str) -> bool {
    serde_json::from_slice::<Value>(&request.body)
        .ok()
        .and_then(|body| body.get("input").and_then(Value::as_array).cloned())
        .is_some_and(|items| {
            items
                .iter()
                .any(|item| item.get("type").and_then(Value::as_str) == Some(input_type))
        })
}

// Test support classifies response item IDs and internal chat-message passthrough as fields to
// remove for semantic assertions. Removing only those fields avoids requiring a fresh child to
// reuse its parent's thread identity; this test does not assert how the backend treats them.
fn cache_prefix_input_projection(request: &ResponsesRequest) -> Vec<Value> {
    strip_response_item_ids_from_json(strip_metadata_from_json(Value::Array(request.input())))
        .as_array()
        .expect("projected request input should remain an array")
        .clone()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fresh_subagent_reuses_root_cache_prefix_until_its_role_boundary() -> Result<()> {
    let server = start_mock_server().await;
    let spawn_args = serde_json::to_string(&json!({
        "message": CHILD_TASK,
        "task_name": "worker",
        "fork_turns": "none",
    }))?;
    let root_request = mount_sse_once_match(
        &server,
        |request: &wiremock::Request| {
            body_contains(request, ROOT_PROMPT)
                && !request_has_input_type(request, "agent_message")
                && !body_contains(request, SPAWN_CALL_ID)
        },
        sse(vec![
            ev_response_created("root-response-1"),
            ev_function_call_with_namespace(
                SPAWN_CALL_ID,
                COLLABORATION_NAMESPACE,
                "spawn_agent",
                &spawn_args,
            ),
            ev_completed("root-response-1"),
        ]),
    )
    .await;
    let child_request = mount_sse_once_match(
        &server,
        |request: &wiremock::Request| {
            body_contains(request, CHILD_TASK) && !body_contains(request, SPAWN_CALL_ID)
        },
        sse(vec![
            ev_response_created("child-response"),
            ev_assistant_message("child-message", "inspection complete"),
            ev_completed("child-response"),
        ]),
    )
    .await;
    mount_sse_once_match(
        &server,
        |request: &wiremock::Request| body_contains(request, SPAWN_CALL_ID),
        sse(vec![
            ev_response_created("root-response-2"),
            ev_assistant_message("root-message", "worker finished"),
            ev_completed("root-response-2"),
        ]),
    )
    .await;

    let mut builder = test_codex()
        .with_model_info_override(MODEL, |model| {
            model.multi_agent_version = Some(codex_protocol::protocol::MultiAgentVersion::V2);
        })
        .with_auth(CodexAuth::from_api_key("dummy"))
        .with_pre_build_hook(|home| {
            std::fs::write(home.join("AGENTS.md"), GLOBAL_INSTRUCTIONS)
                .expect("write stable global instructions");
        })
        .with_config(|config| {
            config.model = Some(MODEL.to_string());
            config.model_reasoning_effort = Some(ReasoningEffort::High);
            config.base_instructions = Some(BASE_INSTRUCTIONS.to_string());
            config.developer_instructions = Some(DEVELOPER_INSTRUCTIONS.to_string());
            config.multi_agent_v2.root_agent_usage_hint_text = Some(ROOT_ROLE_HINT.to_string());
            config.multi_agent_v2.subagent_usage_hint_text = Some(CHILD_ROLE_HINT.to_string());
            config
                .features
                .enable(Feature::Collab)
                .expect("test config should allow feature update");
            config
                .features
                .enable(Feature::MultiAgentV2)
                .expect("test config should allow feature update");
            config
                .features
                .disable(Feature::TokenBudget)
                .expect("test config should allow feature update");
            config
                .features
                .disable(Feature::EnableRequestCompression)
                .expect("test config should allow feature update");
        });
    let test = builder.build_with_auto_env(&server).await?;
    let expected_session_id = test.session_configured.session_id.to_string();
    test.submit_text_turn(ROOT_PROMPT).await?;

    let root_request = root_request
        .requests()
        .into_iter()
        .next()
        .expect("root request");
    let root_thread_id = root_request.header("thread-id").expect("root thread ID");
    let child_request = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(request) = child_request.requests().into_iter().find(|request| {
                request
                    .header("thread-id")
                    .is_some_and(|thread_id| thread_id != root_thread_id)
            }) {
                break request;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| anyhow!("timed out waiting for the child request"))?;
    let child_thread_id = child_request.header("thread-id").expect("child thread ID");

    let root_body = root_request.body_json();
    let child_body = child_request.body_json();
    assert_eq!(root_body["model"], MODEL);
    assert_eq!(child_body["model"], MODEL);
    assert_eq!(root_body["instructions"], BASE_INSTRUCTIONS);
    assert_eq!(child_body["instructions"], BASE_INSTRUCTIONS);
    assert_eq!(root_body["reasoning"]["effort"], "high");
    assert_eq!(child_body["reasoning"], root_body["reasoning"]);
    assert!(
        root_body["tools"]
            .as_array()
            .is_some_and(|tools| !tools.is_empty()),
        "the ordered-tool equality check requires a non-empty tool fixture"
    );
    assert_eq!(child_body["tools"], root_body["tools"]);

    let root_input = cache_prefix_input_projection(&root_request);
    let child_input = cache_prefix_input_projection(&child_request);
    let first_divergence = root_input
        .iter()
        .zip(&child_input)
        .position(|(root, child)| root != child)
        .expect("root and fresh child should deliberately diverge at their role instructions");
    assert!(
        first_divergence > 0,
        "root and fresh child should share startup input before their role instructions"
    );
    assert_eq!(
        &child_input[..first_divergence],
        &root_input[..first_divergence],
        "fresh child should preserve the root's ordered startup input prefix"
    );
    let shared_prefix = serde_json::to_string(&root_input[..first_divergence])?;
    assert!(
        shared_prefix.contains(DEVELOPER_INSTRUCTIONS),
        "shared startup prefix should include stable developer instructions: {shared_prefix}"
    );
    assert!(
        shared_prefix.contains(GLOBAL_INSTRUCTIONS),
        "shared startup prefix should include stable global instructions before the agent-specific boundary: {shared_prefix}"
    );
    assert_eq!(
        json!({
            "root": &root_input[first_divergence],
            "child": &child_input[first_divergence],
        }),
        json!({
            "root": {
                "type": "message",
                "role": "developer",
                "content": [{"type": "input_text", "text": ROOT_ROLE_HINT}],
            },
            "child": {
                "type": "message",
                "role": "developer",
                "content": [{"type": "input_text", "text": CHILD_ROLE_HINT}],
            },
        }),
        "the first unequal items should be the configured root/child role boundaries"
    );
    let child_agent_messages = child_request
        .input()
        .into_iter()
        .filter(|item| item["type"] == "agent_message")
        .map(|item| item["content"].clone())
        .collect::<Vec<_>>();
    assert_eq!(
        child_agent_messages,
        vec![json!([
            {"type": "input_text", "text": "Message Type: NEW_TASK\nTask name: /root/worker\nSender: /root\nPayload:\n"},
            {"type": "encrypted_content", "encrypted_content": CHILD_TASK}
        ])],
        "fresh child should contain its private task exactly once"
    );
    assert!(
        !child_body.to_string().contains(ROOT_PROMPT),
        "fresh child should not receive the parent's private task"
    );
    assert!(
        !child_body.to_string().contains(SPAWN_CALL_ID),
        "fresh child should not receive the parent's spawn call history"
    );

    assert_eq!(
        json!({
            "differentThreadIds": root_thread_id != child_thread_id,
            "root": {
                "sessionId": root_request.header("session-id"),
                "threadId": &root_thread_id,
                "clientRequestId": root_request.header("x-client-request-id"),
                "promptCacheKey": root_body["prompt_cache_key"].clone(),
            },
            "child": {
                "sessionId": child_request.header("session-id"),
                "threadId": &child_thread_id,
                "clientRequestId": child_request.header("x-client-request-id"),
                "promptCacheKey": child_body["prompt_cache_key"].clone(),
            },
        }),
        json!({
            "differentThreadIds": true,
            "root": {
                "sessionId": &expected_session_id,
                "threadId": &root_thread_id,
                "clientRequestId": &root_thread_id,
                "promptCacheKey": &expected_session_id,
            },
            "child": {
                "sessionId": &expected_session_id,
                "threadId": &child_thread_id,
                "clientRequestId": &child_thread_id,
                "promptCacheKey": &expected_session_id,
            },
        })
    );

    Ok(())
}
