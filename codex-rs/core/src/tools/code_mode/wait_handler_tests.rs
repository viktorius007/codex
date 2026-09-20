use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use codex_code_mode::CellId;
use codex_code_mode::CodeModeSession;
use codex_code_mode::CodeModeSessionDelegate;
use codex_code_mode::CodeModeSessionProvider;
use codex_code_mode::CodeModeSessionProviderFuture;
use codex_code_mode::CodeModeSessionResultFuture;
use codex_code_mode::ExecuteRequest;
use codex_code_mode::FunctionCallOutputContentItem;
use codex_code_mode::RuntimeResponse;
use codex_code_mode::StartedCell;
use codex_code_mode::WaitOutcome;
use codex_code_mode::WaitRequest;
use pretty_assertions::assert_eq;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use super::CodeModeWaitHandler;
use super::WAIT_TOOL_NAME;
use crate::session::step_context::StepContext;
use crate::session::tests::make_session_and_context;
use crate::tools::code_mode::CodeModeService;
use crate::tools::context::ToolCallSource;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::registry::ToolExecutor;
use crate::turn_diff_tracker::TurnDiffTracker;

struct SequencedSessionProvider {
    session: Arc<SequencedSession>,
}

impl CodeModeSessionProvider for SequencedSessionProvider {
    fn create_session<'a>(
        &'a self,
        _delegate: Arc<dyn CodeModeSessionDelegate>,
    ) -> CodeModeSessionProviderFuture<'a> {
        let session: Arc<dyn CodeModeSession> = self.session.clone();
        Box::pin(async move { Ok(session) })
    }
}

struct SequencedSession {
    outcomes: async_channel::Receiver<WaitOutcome>,
    wait_count: AtomicUsize,
    wait_started: Notify,
}

impl CodeModeSession for SequencedSession {
    fn execute<'a>(
        &'a self,
        _request: ExecuteRequest,
    ) -> CodeModeSessionResultFuture<'a, StartedCell> {
        Box::pin(async { Err("test session cannot execute cells".to_string()) })
    }

    fn wait<'a>(&'a self, _request: WaitRequest) -> CodeModeSessionResultFuture<'a, WaitOutcome> {
        Box::pin(async move {
            self.wait_count.fetch_add(1, Ordering::SeqCst);
            self.wait_started.notify_one();
            self.outcomes
                .recv()
                .await
                .map_err(|_| "test outcome channel closed".to_string())
        })
    }

    fn terminate<'a>(&'a self, _cell_id: CellId) -> CodeModeSessionResultFuture<'a, WaitOutcome> {
        Box::pin(async { Err("test session cannot terminate cells".to_string()) })
    }

    fn shutdown<'a>(&'a self) -> CodeModeSessionResultFuture<'a, ()> {
        Box::pin(async { Ok(()) })
    }
}

fn yielded(
    content_items: Vec<FunctionCallOutputContentItem>,
    code_mode_host_duration: std::time::Duration,
) -> WaitOutcome {
    WaitOutcome::LiveCell(RuntimeResponse::Yielded {
        cell_id: CellId::new("cell-1".to_string()),
        content_items,
        code_mode_host_duration: Some(code_mode_host_duration),
    })
}

#[tokio::test]
async fn empty_timed_yields_stay_inside_one_wait_until_useful_output() -> anyhow::Result<()> {
    let (outcome_tx, outcome_rx) = async_channel::unbounded();
    let runtime = Arc::new(SequencedSession {
        outcomes: outcome_rx,
        wait_count: AtomicUsize::new(0),
        wait_started: Notify::new(),
    });
    let (mut session, turn) = make_session_and_context().await;
    session.services.code_mode_service = CodeModeService::new(
        Arc::new(SequencedSessionProvider {
            session: Arc::clone(&runtime),
        }),
        &turn.config.code_mode,
        session.services.executed_tool_calls.clone(),
    );
    let session = Arc::new(session);
    let turn = Arc::new(turn);
    let invocation = ToolInvocation {
        session,
        step_context: StepContext::for_test(Arc::clone(&turn)),
        turn,
        cancellation_token: CancellationToken::new(),
        tracker: Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new())),
        call_id: "wait-call".to_string(),
        tool_name: codex_tools::ToolName::plain(WAIT_TOOL_NAME).with_default_namespace(),
        source: ToolCallSource::Direct,
        payload: ToolPayload::Function {
            arguments: r#"{"cell_id":"cell-1","yield_time_ms":1}"#.to_string(),
        },
    };

    outcome_tx
        .send(yielded(Vec::new(), std::time::Duration::from_millis(300)))
        .await?;
    let wait_task = tokio::spawn(async move { CodeModeWaitHandler.handle(invocation).await });
    while runtime.wait_count.load(Ordering::SeqCst) < 2 {
        runtime.wait_started.notified().await;
    }
    assert!(!wait_task.is_finished());

    outcome_tx
        .send(yielded(
            vec![FunctionCallOutputContentItem::InputText {
                text: "partial output".to_string(),
            }],
            std::time::Duration::from_millis(400),
        ))
        .await?;
    let output = tokio::time::timeout(std::time::Duration::from_secs(5), wait_task)
        .await??
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    assert!(output.log_output().contains("partial output"));
    assert!(output.log_output().contains("Wall time 0.7 seconds"));
    assert_eq!(runtime.wait_count.load(Ordering::SeqCst), 2);
    Ok(())
}
