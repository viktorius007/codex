use crate::ExtensionData;
use crate::ExtensionFuture;

/// One retained asynchronous result considered for automatic turn admission.
pub struct AsyncResultAdmissionCandidate<'a> {
    /// Queue-local identity used to select the retained result.
    pub id: u64,
    /// Frozen extension metadata from the turn that launched the async work.
    pub origin_turn_store: &'a ExtensionData,
}

/// Context supplied for one atomic decision over all pending results.
pub struct AsyncResultAdmissionInput<'a> {
    /// All retained results awaiting automatic delivery.
    pub candidates: &'a [AsyncResultAdmissionCandidate<'a>],
    /// Extension metadata for the destination thread.
    pub thread_store: &'a ExtensionData,
}

/// One contributor's batch admission decision.
#[derive(Default)]
pub struct AsyncResultAdmissionDecision {
    /// Candidate identities this contributor prevents from starting a turn.
    pub denied_ids: Vec<u64>,
    /// Optional guard held until Core commits or cancels the admitted start.
    pub permit: Option<Box<dyn Send>>,
}

/// Decides which retained asynchronous results may start a model turn.
pub trait AsyncResultAdmissionContributor: Send + Sync {
    fn decide<'a>(
        &'a self,
        input: AsyncResultAdmissionInput<'a>,
    ) -> ExtensionFuture<'a, AsyncResultAdmissionDecision>;
}
