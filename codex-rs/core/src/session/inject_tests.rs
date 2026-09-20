use crate::session::TurnInput;
use crate::session::tests::make_session_and_context_with_rx;
use crate::state::ActiveTurn;
use codex_features::Feature;
use codex_history::CodexHarnessMetadata;
use codex_history::ResponseItemEnvelope;
use codex_history::RolloutItem;
use codex_protocol::models::ConfigurationReasoning;
use codex_protocol::models::ResponseItem;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::protocol::EventMsg;
use pretty_assertions::assert_eq;
use std::sync::Arc;

#[tokio::test]
async fn completion_arriving_during_turn_retirement_is_retained_once_for_the_next_turn() {
    let (session, turn_context, _rx_event) = make_session_and_context_with_rx().await;
    let retiring_turn = ActiveTurn::default();
    let retiring_turn_state = Arc::clone(&retiring_turn.turn_state);
    *session.active_turn.lock().await = Some(retiring_turn);

    session
        .inject_or_start(
            vec![ResponseItem::Other],
            Arc::clone(&turn_context.extension_data),
        )
        .await;

    assert_eq!(
        session
            .input_queue
            .take_pending_input_for_turn_state(retiring_turn_state.as_ref())
            .await,
        Vec::<TurnInput>::new(),
        "a retiring turn must not own a late completion"
    );
    assert_eq!(
        session
            .input_queue
            .get_pending_input(&session.active_turn)
            .await
            .0,
        Vec::<TurnInput>::new(),
        "the retiring turn's next-step drain must not claim the completion"
    );
    let origins = session.input_queue.pending_async_result_origins().await;
    assert_eq!(origins.len(), 1, "the late completion must remain retained");
    let retained = session
        .input_queue
        .claim_async_results(&[origins[0].0])
        .await;
    assert_eq!(
        retained[0].input(),
        vec![TurnInput::ResponseItem(ResponseItemEnvelope::new(
            ResponseItem::Other
        ))]
        .as_slice()
    );
    assert_eq!(
        session.input_queue.drain_next_async_results().await,
        Vec::<TurnInput>::new(),
        "the retained completion must be consumed exactly once"
    );
}

#[tokio::test]
async fn cancelled_batch_restores_each_result_with_its_exact_origin() {
    let (session, first_turn, _rx_event) = make_session_and_context_with_rx().await;
    let second_turn = session.new_default_turn().await;
    session
        .input_queue
        .enqueue_async_result(
            vec![TurnInput::ResponseItem(ResponseItemEnvelope::new(
                ResponseItem::Other,
            ))],
            Arc::clone(&first_turn.extension_data),
        )
        .await;
    session
        .input_queue
        .enqueue_async_result(
            vec![TurnInput::ResponseItem(ResponseItemEnvelope::new(
                ResponseItem::Other,
            ))],
            Arc::clone(&second_turn.extension_data),
        )
        .await;
    let origins = session.input_queue.pending_async_result_origins().await;
    let ids = origins.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    let claimed = session.input_queue.claim_async_results(&ids).await;

    session.input_queue.restore_async_results(claimed).await;

    let restored = session.input_queue.pending_async_result_origins().await;
    assert_eq!(restored.iter().map(|(id, _)| *id).collect::<Vec<_>>(), ids);
    assert!(Arc::ptr_eq(&restored[0].1, &first_turn.extension_data));
    assert!(Arc::ptr_eq(&restored[1].1, &second_turn.extension_data));
}

#[tokio::test]
async fn completion_after_interruption_is_retained_without_restarting_the_model() {
    let (session, turn_context, _rx_event) = make_session_and_context_with_rx().await;
    *session.active_turn.lock().await = Some(ActiveTurn::default());

    session
        .inject_or_start(
            vec![ResponseItem::Other],
            Arc::clone(&turn_context.extension_data),
        )
        .await;
    session
        .abort_all_tasks(codex_protocol::protocol::TurnAbortReason::Interrupted)
        .await;

    assert!(
        session
            .active_turn
            .lock()
            .await
            .as_ref()
            .is_some_and(|turn| turn.cancelled_before_start),
        "interruption must cancel a reserved automatic start"
    );
    assert_eq!(
        session.input_queue.drain_next_async_results().await,
        vec![TurnInput::ResponseItem(ResponseItemEnvelope::new(
            ResponseItem::Other
        ))]
    );
}

#[tokio::test]
async fn harness_authored_configuration_updates_preserve_metadata_and_resume() {
    let (session, turn_context, rx_event) = make_session_and_context_with_rx().await;
    assert!(!session.enabled(Feature::RetainClientDeveloperMessages));

    let expected = ResponseItemEnvelope {
        item: ResponseItem::ConfigurationUpdate {
            reasoning: ConfigurationReasoning {
                effort: ReasoningEffort::High,
            },
        },
        metadata: Some(CodexHarnessMetadata {
            harness_authored_configuration: true,
            ..Default::default()
        }),
    };
    session
        .record_annotated_conversation_items(
            &turn_context,
            turn_context.model_info(),
            vec![expected.clone()],
        )
        .await;

    let recorded = session.clone_history().await.into_annotated_items();
    assert_eq!(recorded, vec![expected.clone()]);
    let mut raw_items = Vec::new();
    while let Ok(event) = rx_event.try_recv() {
        if let EventMsg::RawResponseItem(event) = event.msg {
            raw_items.push(event.item);
        }
    }
    assert_eq!(raw_items, vec![expected.item]);

    let rollout_items = recorded
        .iter()
        .cloned()
        .map(RolloutItem::ResponseItem)
        .collect::<Vec<_>>();
    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;
    assert_eq!(reconstructed.history, recorded);
}
