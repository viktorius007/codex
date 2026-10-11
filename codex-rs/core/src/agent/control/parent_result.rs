use super::LocalAgentControl;
use crate::codex_thread::CodexThread;
use codex_protocol::ThreadId;
use codex_protocol::error::CodexErr;
use codex_protocol::error::CodexErrorDetails;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_thread_store::ReadThreadParams;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
use tokio::sync::OwnedRwLockReadGuard;

type ParentPin = Mutex<Option<OwnedRwLockReadGuard<()>>>;

/// Keeps a child turn's direct parent resident until terminal delivery or removal.
pub(crate) struct ParentResultResidencyGuard {
    _pin: Arc<ParentPin>,
}

#[derive(Default)]
struct ParentResultResidencyPins(Mutex<Vec<Weak<ParentPin>>>);

impl ParentResultResidencyPins {
    fn register(&self, pin: &Arc<ParentPin>) {
        let mut pins = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        pins.retain(|pin| pin.strong_count() > 0);
        pins.push(Arc::downgrade(pin));
    }

    fn cancel_all(&self) {
        for pin in self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .drain(..)
        {
            if let Some(pin) = pin.upgrade() {
                pin.lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .take();
            }
        }
    }
}

pub(crate) fn cancel_parent_result_residency(thread: &Arc<CodexThread>) {
    if let Some(pins) = thread
        .session
        .services
        .thread_extension_data
        .get::<ParentResultResidencyPins>()
    {
        pins.cancel_all();
    }
}

impl LocalAgentControl {
    pub(crate) async fn parent_result_residency(
        &self,
        child_id: ThreadId,
        version: MultiAgentVersion,
        source: &SessionSource,
    ) -> CodexResult<Option<ParentResultResidencyGuard>> {
        let SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id, ..
        }) = source
        else {
            return Ok(None);
        };
        if version != MultiAgentVersion::V2 {
            return Ok(None);
        }
        let state = self.runtime.upgrade()?;
        let mut missing = Vec::new();
        let mut seen = HashSet::new();
        let mut ancestor_id = *parent_thread_id;
        let mut parent = loop {
            if !seen.insert(ancestor_id) {
                return Err(CodexErr::InvalidRequest(format!(
                    "cannot restore result parent {parent_thread_id}: cyclic stored parent lineage"
                )));
            }
            match state.get_thread(ancestor_id).await {
                Ok(parent) => break parent,
                Err(err) if matches!(err.details(), CodexErrorDetails::ThreadNotFound(_)) => {}
                Err(err) => return Err(err),
            }
            let stored = state
                .read_stored_thread(ReadThreadParams {
                    thread_id: ancestor_id,
                    include_archived: true,
                    include_history: false,
                })
                .await?;
            missing.push(ancestor_id);
            ancestor_id = stored
                .parent_thread_id
                .ok_or(CodexErr::ThreadNotFound(ancestor_id))?;
        };
        let mut restoration_pins = Vec::new();
        let mut direct_pin = self.runtime.pin_v2_residency(&state, &parent).await?;
        for descendant_id in missing.into_iter().rev() {
            restoration_pins.push(direct_pin.take());
            let config = parent.session.get_config().await.as_ref().clone();
            self.ensure_v2_agent_loaded(config, descendant_id, Some(parent))
                .await?;
            parent = state.get_thread(descendant_id).await?;
            direct_pin = self.runtime.pin_v2_residency(&state, &parent).await?;
        }
        let Some(direct_pin) = direct_pin else {
            return Ok(None);
        };
        let pin = Arc::new(Mutex::new(Some(direct_pin)));
        // Removal holds the write lock while cancelling, so registration cannot escape it.
        let threads = state.threads.read().await;
        let child = threads
            .get(&child_id)
            .ok_or(CodexErr::ThreadNotFound(child_id))?;
        child
            .session
            .services
            .thread_extension_data
            .get_or_init(ParentResultResidencyPins::default)
            .register(&pin);
        Ok(Some(ParentResultResidencyGuard { _pin: pin }))
    }
}
