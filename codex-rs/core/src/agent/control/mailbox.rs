//! Retains communications and asynchronous results while local sessions are unloaded.
//! Eviction transfers mail before releasing the recipient's residency guard.

use crate::PendingAgentMessage;
use crate::agent::api::AgentMailboxDelivery;
use crate::agent_communication::PENDING_MAILBOX_MESSAGES;
use codex_diagnostics::GaugeGuard;
use codex_protocol::ThreadId;
use codex_protocol::error::CodexErr;
use codex_protocol::error::Result;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::PoisonError;
use tokio::sync::watch;

pub(super) struct Mailboxes(Mutex<Option<HashMap<ThreadId, Mailbox>>>);

impl Default for Mailboxes {
    fn default() -> Self {
        Self(Mutex::new(Some(HashMap::new())))
    }
}

#[derive(Default)]
struct Mailbox {
    pending: Vec<StoredMail>,
    state: watch::Sender<bool>,
}

struct StoredMail {
    id: Option<String>,
    delivery: AgentMailboxDelivery,
    _diagnostics_guard: GaugeGuard,
}

impl Mailboxes {
    pub(super) fn enqueue(
        &self,
        agent: ThreadId,
        id: Option<String>,
        messages: Vec<PendingAgentMessage>,
    ) -> Result<()> {
        self.enqueue_deliveries(
            agent,
            id,
            messages
                .into_iter()
                .map(|communication| {
                    AgentMailboxDelivery::communication(communication, Default::default(), None)
                })
                .collect(),
        )
    }

    pub(super) fn enqueue_deliveries(
        &self,
        agent: ThreadId,
        id: Option<String>,
        deliveries: Vec<AgentMailboxDelivery>,
    ) -> Result<()> {
        let mut mailboxes = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let mailbox = mailboxes
            .as_mut()
            .ok_or_else(|| CodexErr::InvalidRequest("agent runtime is shutting down".into()))?
            .entry(agent)
            .or_default();
        mailbox
            .pending
            .extend(deliveries.into_iter().map(|delivery| StoredMail {
                id: id.clone(),
                delivery,
                _diagnostics_guard: PENDING_MAILBOX_MESSAGES.track(),
            }));
        mailbox.state.send_replace(!mailbox.pending.is_empty());
        Ok(())
    }

    pub(super) fn take(&self, agent: ThreadId) -> Vec<AgentMailboxDelivery> {
        let mut mailboxes = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(mailbox) = mailboxes
            .as_mut()
            .and_then(|mailboxes| mailboxes.get_mut(&agent))
        else {
            return Vec::new();
        };
        let pending = std::mem::take(&mut mailbox.pending);
        mailbox.state.send_if_modified(std::mem::take);
        pending
            .into_iter()
            .map(|mail| {
                // Mail transferred during eviction was already received by the old session.
                if let Some(id) = mail.id {
                    crate::agent_communication::emit_agent_communication_receive(&id);
                }
                mail.delivery
            })
            .collect()
    }

    pub(super) fn watch(&self, agent: ThreadId) -> watch::Receiver<bool> {
        let mut mailboxes = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match mailboxes.as_mut() {
            Some(mailboxes) => {
                // Private delegates and cancelled startups can stop without explicit removal.
                // Retain unread mail and live subscriptions, but reclaim abandoned empty entries.
                mailboxes.retain(|_, mailbox| {
                    !mailbox.pending.is_empty() || mailbox.state.receiver_count() != 0
                });
                mailboxes.entry(agent).or_default().state.subscribe()
            }
            None => watch::channel(/*init*/ false).1,
        }
    }

    pub(super) fn remove(&self, agent: ThreadId) {
        // Dropping pending mail can run extension destructors. Release the lock first.
        let mailbox = self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_mut()
            .and_then(|mailboxes| mailboxes.remove(&agent));
        drop(mailbox);
    }

    pub(super) fn close(&self) {
        // Detach under the lock, then notify readers and drop pending mail outside it.
        let mailboxes = self.0.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(mailboxes) = mailboxes {
            for mailbox in mailboxes.values() {
                mailbox.state.send_replace(false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::api::AgentMailboxPayload;
    use crate::session::TurnInput;
    use codex_extension_api::ExtensionData;
    use codex_protocol::models::ResponseItem;
    use std::sync::Arc;

    #[test]
    fn async_mail_preserves_payload_origin_and_is_taken_once() {
        let mailboxes = Mailboxes::default();
        let agent = ThreadId::new();
        let updates = mailboxes.watch(agent);
        let origin = Arc::new(ExtensionData::new("origin-turn"));
        let input = vec![TurnInput::ResponseItem(ResponseItem::Other.into())];
        mailboxes
            .enqueue_deliveries(
                agent,
                None,
                vec![AgentMailboxDelivery::async_result(
                    input.clone(),
                    Arc::clone(&origin),
                )],
            )
            .expect("enqueue async result");
        assert!(*updates.borrow());
        let mut deliveries = mailboxes.take(agent);
        assert_eq!(deliveries.len(), 1);
        let AgentMailboxDelivery(AgentMailboxPayload::AsyncResult {
            input: delivered,
            origin: delivered_origin,
        }) = deliveries.pop().unwrap()
        else {
            panic!("expected async result");
        };
        assert_eq!(delivered, input);
        assert!(Arc::ptr_eq(&delivered_origin, &origin));
        assert!(!*updates.borrow());
        assert!(mailboxes.take(agent).is_empty());
    }
}
