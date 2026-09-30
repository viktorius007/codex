use super::*;

use crate::tools::router::ToolCatalogDigest;
use codex_protocol::dynamic_tools::DynamicToolFunctionSpec;
use codex_protocol::dynamic_tools::DynamicToolNamespaceSpec;
use codex_protocol::dynamic_tools::DynamicToolNamespaceTool;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::protocol::ToolCatalogChangedEvent;
use pretty_assertions::assert_eq;
use tokio_util::sync::CancellationToken;

fn dynamic_tool(name: &str, description: &str) -> DynamicToolSpec {
    DynamicToolSpec::Function(DynamicToolFunctionSpec {
        name: name.to_string(),
        description: description.to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false,
        }),
        defer_loading: false,
    })
}

fn registration_noise_namespace(names: [&str; 2]) -> DynamicToolSpec {
    DynamicToolSpec::Namespace(DynamicToolNamespaceSpec {
        name: "fixture".to_string(),
        description: "Registration-order fixture.".to_string(),
        tools: names
            .into_iter()
            .map(|name| {
                DynamicToolNamespaceTool::Function(DynamicToolFunctionSpec {
                    name: name.to_string(),
                    description: format!("{name} description"),
                    input_schema: json!({
                        "type": "object",
                        "properties": {},
                        "additionalProperties": false,
                    }),
                    defer_loading: false,
                })
            })
            .collect(),
    })
}

fn baseline_tools() -> Vec<DynamicToolSpec> {
    vec![
        dynamic_tool("stable", "unchanged"),
        dynamic_tool("mutating", "before"),
        dynamic_tool("leaving", "gone"),
        registration_noise_namespace(["alpha", "beta"]),
    ]
}

fn reordered_baseline_tools() -> Vec<DynamicToolSpec> {
    vec![
        dynamic_tool("stable", "unchanged"),
        dynamic_tool("mutating", "before"),
        dynamic_tool("leaving", "gone"),
        registration_noise_namespace(["beta", "alpha"]),
    ]
}

fn changed_tools() -> Vec<DynamicToolSpec> {
    vec![
        dynamic_tool("stable", "unchanged"),
        dynamic_tool("mutating", "after"),
        dynamic_tool("arriving", "new"),
        registration_noise_namespace(["alpha", "beta"]),
    ]
}

async fn turn_with_tools(
    session: &Session,
    dynamic_tools: Vec<DynamicToolSpec>,
) -> Arc<TurnContext> {
    let mut turn = session.new_default_turn().await;
    Arc::get_mut(&mut turn)
        .expect("new turn should be unshared")
        .dynamic_tools = dynamic_tools;
    turn
}

async fn select_step(session: &Arc<Session>, turn: Arc<TurnContext>) -> Arc<StepContext> {
    session
        .capture_step_context(turn, &CancellationToken::new())
        .await
        .expect("selected step should capture")
}

fn tool_catalog_events(events: &async_channel::Receiver<Event>) -> Vec<serde_json::Value> {
    let mut catalog_events = Vec::new();
    while let Ok(event) = events.try_recv() {
        if let EventMsg::ToolCatalogChanged(event) = event.msg {
            catalog_events.push(
                serde_json::to_value(EventMsg::ToolCatalogChanged(event))
                    .expect("catalog event should serialize"),
            );
        }
    }
    catalog_events
}

fn persisted_tool_catalog_events(items: &[RolloutItem]) -> Vec<serde_json::Value> {
    items
        .iter()
        .filter_map(|item| match item {
            RolloutItem::EventMsg(EventMsg::ToolCatalogChanged(event)) => Some(
                serde_json::to_value(EventMsg::ToolCatalogChanged(event.clone()))
                    .expect("catalog event should serialize"),
            ),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn selected_tool_catalog_change_persists_once_and_restores_digest_on_resume() {
    let (mut session, baseline_turn, events) =
        make_session_and_context_with_dynamic_tools_and_rx(baseline_tools()).await;
    let rollout_path = attach_thread_persistence(
        Arc::get_mut(&mut session).expect("test session should still be uniquely owned"),
    )
    .await;

    let baseline = select_step(&session, baseline_turn).await;
    let baseline_digest = ToolCatalogDigest::new(&baseline.tool_router.model_visible_specs());
    assert_eq!(
        tool_catalog_events(&events),
        Vec::<serde_json::Value>::new()
    );

    let speculative_turn = turn_with_tools(&session, changed_tools()).await;
    session
        .capture_speculative_step_context(speculative_turn, &CancellationToken::new())
        .await
        .expect("speculative step should capture");
    assert_eq!(
        tool_catalog_events(&events),
        Vec::<serde_json::Value>::new()
    );

    let reordered_turn = turn_with_tools(&session, reordered_baseline_tools()).await;
    let reordered = select_step(&session, reordered_turn).await;
    assert_eq!(
        ToolCatalogDigest::new(&reordered.tool_router.model_visible_specs()),
        baseline_digest,
    );
    assert_eq!(
        tool_catalog_events(&events),
        Vec::<serde_json::Value>::new()
    );

    let changed_turn = turn_with_tools(&session, changed_tools()).await;
    let changed = select_step(&session, changed_turn).await;
    let changed_digest = ToolCatalogDigest::new(&changed.tool_router.model_visible_specs());
    let expected = EventMsg::ToolCatalogChanged(ToolCatalogChangedEvent {
        previous_digest: baseline_digest.catalog.clone(),
        current_digest: changed_digest.catalog.clone(),
        added: vec!["arriving".to_string()],
        removed: vec!["leaving".to_string()],
        changed: vec!["mutating".to_string()],
    });
    let expected = serde_json::to_value(expected).expect("expected event should serialize");
    assert_eq!(tool_catalog_events(&events), vec![expected.clone()]);

    session.flush_rollout().await.expect("rollout should flush");
    let InitialHistory::Resumed(resumed) = RolloutRecorder::get_rollout_history(&rollout_path)
        .await
        .expect("rollout should be readable")
    else {
        panic!("expected resumed rollout history");
    };
    assert_eq!(
        persisted_tool_catalog_events(&resumed.history),
        vec![expected]
    );

    let serialized_history =
        serde_json::to_value(resumed.history.as_ref()).expect("rollout history should serialize");
    let resumed_history =
        serde_json::from_value(serialized_history).expect("rollout history should deserialize");
    session.state.lock().await.last_tool_catalog_digest = None;
    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: session.thread_id,
            history: Arc::new(resumed_history),
            rollout_path: Some(rollout_path),
        }))
        .await;

    assert_eq!(
        session
            .state
            .lock()
            .await
            .last_tool_catalog_digest
            .as_ref()
            .map(|digest| digest.catalog.clone()),
        Some(changed_digest.catalog),
    );
}
