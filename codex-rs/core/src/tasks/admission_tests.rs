use super::SessionTask;
use super::SessionTaskResult;
use crate::session::TurnInput;
use crate::session::session::Session;
use crate::session::tests::make_session_and_context_with_rx;
use crate::session::turn_context::TurnContext;
use crate::state::ActiveTurn;
use crate::state::TaskKind;
use codex_history::ResponseItemEnvelope;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::TurnAbortReason;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

struct WaitingTask(Mutex<Option<oneshot::Sender<Vec<TurnInput>>>>);

impl SessionTask for WaitingTask {
    fn kind(&self) -> TaskKind {
        TaskKind::Regular
    }
    fn span_name(&self) -> &'static str {
        "admission_test"
    }
    async fn run(
        self: Arc<Self>,
        _session: Arc<Session>,
        _ctx: Arc<TurnContext>,
        input: Vec<TurnInput>,
        cancellation_token: CancellationToken,
    ) -> SessionTaskResult {
        self.0.lock().unwrap().take().unwrap().send(input).unwrap();
        cancellation_token.cancelled().await;
        Ok(None)
    }
}

#[tokio::test]
async fn replaced_async_reservation_preserves_the_explicit_task_and_pending_input() {
    let (session, explicit_context, _rx_event) = make_session_and_context_with_rx().await;
    let reservation = ActiveTurn::default();
    let reserved_state = Arc::clone(&reservation.turn_state);
    *session.active_turn.lock().await = Some(reservation);
    let async_context = session.new_default_turn().await;
    let (received_tx, received_rx) = oneshot::channel();
    session
        .start_task(
            Arc::clone(&explicit_context),
            Vec::new(),
            WaitingTask(Mutex::new(Some(received_tx))),
        )
        .await;
    received_rx.await.unwrap();
    let pending_input = vec![TurnInput::ResponseItem(ResponseItemEnvelope::new(
        ResponseItem::Other,
    ))];
    session
        .input_queue
        .extend_pending_input_for_turn_state(reserved_state.as_ref(), pending_input.clone())
        .await;
    let (unused_received, _unused_rx) = oneshot::channel();
    assert!(
        !session
            .start_task_with_reservation(
                async_context,
                Vec::new(),
                WaitingTask(Mutex::new(Some(unused_received))),
                Some(&reserved_state),
                Vec::new()
            )
            .await
    );
    assert_eq!(
        session
            .active_turn
            .lock()
            .await
            .as_ref()
            .unwrap()
            .task
            .as_ref()
            .unwrap()
            .turn_context
            .sub_id,
        explicit_context.sub_id
    );
    assert_eq!(
        session
            .input_queue
            .take_pending_input_for_turn_state(reserved_state.as_ref())
            .await,
        pending_input
    );
    session.abort_all_tasks(TurnAbortReason::Replaced).await;
}

#[tokio::test]
async fn explicit_start_preserves_input_when_an_automatic_task_registered_first() {
    let (session, automatic_context, _rx_event) = make_session_and_context_with_rx().await;
    let explicit_context = session.new_default_turn().await;
    let (automatic_received, automatic_received_rx) = oneshot::channel();
    session
        .start_task(
            automatic_context,
            Vec::new(),
            WaitingTask(Mutex::new(Some(automatic_received))),
        )
        .await;
    automatic_received_rx.await.unwrap();
    let (explicit_received, explicit_received_rx) = oneshot::channel();
    let explicit_input = vec![TurnInput::ResponseItem(ResponseItemEnvelope::new(
        ResponseItem::Other,
    ))];
    session
        .start_task(
            Arc::clone(&explicit_context),
            explicit_input.clone(),
            WaitingTask(Mutex::new(Some(explicit_received))),
        )
        .await;
    assert_eq!(explicit_received_rx.await.unwrap(), explicit_input);
    assert_eq!(
        session
            .active_turn
            .lock()
            .await
            .as_ref()
            .unwrap()
            .task
            .as_ref()
            .unwrap()
            .turn_context
            .sub_id,
        explicit_context.sub_id
    );
    session.abort_all_tasks(TurnAbortReason::Replaced).await;
}

#[tokio::test]
async fn replaced_idle_reservation_cannot_be_reused_by_a_stale_async_start() {
    let (session, turn_context, _rx_event) = make_session_and_context_with_rx().await;
    let reserved_state = Arc::clone(&ActiveTurn::default().turn_state);
    let replacement = ActiveTurn::default();
    let replacement_state = Arc::clone(&replacement.turn_state);
    *session.active_turn.lock().await = Some(replacement);
    let (received, _received_rx) = oneshot::channel();
    assert!(
        !session
            .start_task_with_reservation(
                turn_context,
                Vec::new(),
                WaitingTask(Mutex::new(Some(received))),
                Some(&reserved_state),
                Vec::new(),
            )
            .await
    );
    let active = session.active_turn.lock().await;
    let replacement = active.as_ref().unwrap();
    assert!(Arc::ptr_eq(&replacement.turn_state, &replacement_state));
    assert!(replacement.task.is_none());
}
