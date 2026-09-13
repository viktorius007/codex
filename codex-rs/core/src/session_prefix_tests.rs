use codex_protocol::AgentPath;
use codex_protocol::protocol::AgentStatus;
use codex_utils_output_truncation::approx_token_count;

use super::COMPLETION_MESSAGE_MAX_TOKENS;
use super::ERROR_NEXT_ACTION;
use super::format_inter_agent_completion_message;

const EXPECTED_COMPLETION_MESSAGE_MAX_TOKENS: usize = 1_000;

#[test]
fn error_completion_message_stays_below_manual_review_threshold() {
    let message = format_inter_agent_completion_message(
        AgentPath::root(),
        AgentPath::try_from("/root/worker").expect("valid agent path"),
        &AgentStatus::Errored("stream disconnected ".repeat(1_000)),
    )
    .expect("error status should produce a completion message");

    assert!(approx_token_count(&message) < COMPLETION_MESSAGE_MAX_TOKENS);
    assert!(message.contains(ERROR_NEXT_ACTION));
}

#[test]
fn successful_completion_message_is_bounded_and_marks_truncation() {
    let payload = format!(
        "success-prefix-{}-success-suffix",
        "subagent completion output ".repeat(2_000)
    );
    let message = format_inter_agent_completion_message(
        AgentPath::root(),
        AgentPath::try_from("/root/worker").expect("valid agent path"),
        &AgentStatus::Completed(Some(payload)),
    )
    .expect("completed status should produce a completion message");

    assert!(
        approx_token_count(&message) <= EXPECTED_COMPLETION_MESSAGE_MAX_TOKENS,
        "successful completion envelope must stay within the 1,000-token bound"
    );
    assert!(message.starts_with(
        "Message Type: FINAL_ANSWER\nTask name: /root\nSender: /root/worker\nPayload:\nsuccess-prefix-"
    ));
    assert!(message.contains("tokens truncated"));
    assert!(message.ends_with("-success-suffix"));
}
