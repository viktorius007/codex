use codex_protocol::models::ContentItemKind;
use codex_utils_output_truncation::TruncationPolicy;
use codex_utils_output_truncation::truncate_text;

use super::ContextualUserFragment;

const EXEC_COMPLETION_TOKENS: usize = 1_000;
// The truncation marker is added outside the requested content budget.
const TRUNCATION_MARKER_TOKEN_RESERVE: usize = 32;

/// Model-visible completion of a unified exec process that previously yielded.
pub(crate) struct ExecCompletion {
    body: String,
}

impl ExecCompletion {
    pub(crate) fn new(
        call_id: &str,
        process_id: i32,
        command: &[String],
        exit_code: i32,
        output: &str,
    ) -> Self {
        let command = command.join(" ");
        Self {
            body: truncate_text(
                &format!(
                    "<exec-command-completed call-id=\"{call_id}\" process-id=\"{process_id}\" exit-code=\"{exit_code}\">\n<command>{command}</command>\n<output>\n{output}\n</output>\n</exec-command-completed>"
                ),
                TruncationPolicy::Tokens(EXEC_COMPLETION_TOKENS - TRUNCATION_MARKER_TOKEN_RESERVE),
            ),
        }
    }
}

impl ContextualUserFragment for ExecCompletion {
    fn role(&self) -> &'static str {
        "user"
    }

    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind("exec.completion".to_string())
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn body(&self) -> String {
        self.body.clone()
    }

    fn type_markers() -> (&'static str, &'static str) {
        ("", "")
    }
}

#[cfg(test)]
#[path = "exec_completion_tests.rs"]
mod tests;
