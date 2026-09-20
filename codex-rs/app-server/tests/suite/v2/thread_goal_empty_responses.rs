//! Exercises the goal extension's empty-response breaker through the public API.

use anyhow::Result;
use app_test_support::MockResponsesConfig;
use app_test_support::TestAppServer;
use app_test_support::create_mock_responses_server_sequence;
use codex_app_server_protocol::ThreadGoalGetResponse;
use codex_app_server_protocol::ThreadGoalSetResponse;
use codex_app_server_protocol::ThreadGoalStatus;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::ThreadStartResponse;
use codex_app_server_protocol::TurnCompletedNotification;
use codex_app_server_protocol::TurnStatus;
use codex_app_server_protocol::WarningNotification;
use codex_features::Feature;
use core_test_support::responses;
use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;
use tokio::time::Duration;
use tokio::time::timeout;

#[test_case::test_case("empty"; "empty final")]
#[test_case::test_case("missing_final"; "missing final")]
#[test_case::test_case("final_answer"; "nonempty final answer")]
#[test_case::test_case("commentary"; "nonempty commentary")]
#[test_case::test_case("tool"; "tool only")]
#[test_case::test_case("polling"; "nonempty polling output")]
#[tokio::test]
async fn goal_continuations_block_after_three_without_model_output(
    response_kind: &str,
) -> Result<()> {
    let turns = if matches!(response_kind, "final_answer" | "commentary" | "polling") {
        6
    } else {
        3
    };
    let mut scripts = Vec::new();
    for turn in 1..=turns {
        let mut id = format!("response-{turn}");
        let mut events = vec![responses::ev_response_created(&id)];
        if turn == 3 {
            if response_kind == "tool" {
                events.push(responses::ev_function_call("get-goal", "get_goal", "{}"));
                events.push(responses::ev_completed(&id));
                scripts.push(responses::sse(events));
                id.push_str("-after-tool");
                events = vec![responses::ev_response_created(&id)];
            } else if matches!(response_kind, "final_answer" | "commentary" | "polling") {
                let text = if response_kind == "polling" {
                    "Still waiting for the process."
                } else {
                    "Useful progress"
                };
                let mut added = responses::ev_assistant_message("progress", "");
                added["type"] = json!("response.output_item.added");
                events.push(added);
                events.push(responses::ev_output_text_delta(text));
                let mut event = responses::ev_assistant_message("progress", text);
                event["item"]["phase"] = json!(if response_kind == "polling" {
                    "final_answer"
                } else {
                    response_kind
                });
                events.push(event);
            }
        }
        if response_kind != "missing_final" {
            let mut empty_final =
                responses::ev_assistant_message(&format!("empty-final-{turn}"), "");
            empty_final["item"]["phase"] = json!("final_answer");
            events.push(empty_final);
        }
        events.push(responses::ev_completed(&id));
        scripts.push(responses::sse(events));
    }
    let server = create_mock_responses_server_sequence(scripts).await;
    let codex_home = TempDir::new()?;
    MockResponsesConfig::new(&server.uri())
        .with_model("gpt-5.4")
        .enable_feature(Feature::Goals)
        .write(codex_home.path())?;
    let mut mcp = TestAppServer::builder()
        .with_codex_home(codex_home.path())
        .without_managed_config()
        .build_initialized()
        .await?;
    let request = mcp
        .send_thread_start_request_with_auto_env(ThreadStartParams::default())
        .await?;
    let ThreadStartResponse { thread, .. } = mcp.read_response(request).await?;
    let request = mcp
        .send_raw_request(
            "thread/goal/set",
            Some(json!({"threadId": thread.id, "objective": "Finish the work"})),
        )
        .await?;
    let _: ThreadGoalSetResponse = mcp.read_response(request).await?;

    for turn in 1..=turns {
        if turn == 3 && matches!(response_kind, "final_answer" | "commentary" | "polling") {
            let delta: serde_json::Value = timeout(
                Duration::from_secs(30),
                mcp.read_notification("item/agentMessage/delta"),
            )
            .await??;
            let expected = if response_kind == "polling" {
                "Still waiting for the process."
            } else {
                "Useful progress"
            };
            assert_eq!(json!(expected), delta["delta"]);
        }
        let completed: TurnCompletedNotification = timeout(
            Duration::from_secs(30),
            mcp.read_notification("turn/completed"),
        )
        .await??;
        assert_eq!(TurnStatus::Completed, completed.turn.status);
        assert_eq!(None, completed.turn.error);
    }
    let request = mcp
        .send_raw_request("thread/goal/get", Some(json!({"threadId": thread.id})))
        .await?;
    let result: ThreadGoalGetResponse = mcp.read_response(request).await?;
    let goal = result.goal.expect("goal exists");
    assert_eq!(ThreadGoalStatus::Blocked, goal.status);
    assert_eq!("Finish the work", goal.objective);
    let warning: WarningNotification = timeout(Duration::from_secs(30), async {
        loop {
            let warning: WarningNotification = mcp.read_notification("warning").await?;
            if warning.message.starts_with("Goal blocked after three") {
                break anyhow::Ok(warning);
            }
        }
    })
    .await??;
    assert_eq!(Some(thread.id), warning.thread_id);
    assert_eq!(
        "Goal blocked after three automatic continuation turns produced no model output. The goal was preserved; set its status to active to resume it.",
        warning.message,
    );
    let request_count = server
        .received_requests()
        .await
        .expect("mock server should record requests")
        .into_iter()
        .filter(|request| request.url.path() == "/v1/responses")
        .count();
    assert_eq!(turns + usize::from(response_kind == "tool"), request_count);
    server.verify().await;
    Ok(())
}
