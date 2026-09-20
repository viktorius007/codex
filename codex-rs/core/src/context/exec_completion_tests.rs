use super::ExecCompletion;
use crate::context::ContextualUserFragment;
use codex_utils_output_truncation::approx_token_count;
use pretty_assertions::assert_eq;

#[test]
fn completion_preserves_small_result() {
    let completion = ExecCompletion::new("call-1", 42, &["echo".into(), "done".into()], 0, "done");
    assert_eq!(
        completion.body(),
        "<exec-command-completed call-id=\"call-1\" process-id=\"42\" exit-code=\"0\">\n<command>echo done</command>\n<output>\ndone\n</output>\n</exec-command-completed>"
    );
}

#[test]
fn completion_bounds_command_and_output_together() {
    for (command, output) in [
        ("x".repeat(16_000), "done".into()),
        ("echo".into(), "x".repeat(16_000)),
    ] {
        let body = ExecCompletion::new("call-1", 42, &[command], 7, &output).body();
        assert!(
            approx_token_count(&body) <= 1_000,
            "entire completion must fit the context limit"
        );
        assert!(body.starts_with(
            "<exec-command-completed call-id=\"call-1\" process-id=\"42\" exit-code=\"7\">"
        ));
        assert!(body.ends_with("</output>\n</exec-command-completed>"));
        assert!(body.contains("tokens truncated"));
    }
}
