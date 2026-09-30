# TODO

- [ ] Add an explicit way for an active Goal to defer automatic continuation until one finite `exec_command` process exits. While armed, make no Goal continuation model calls; on exit, failure, timeout, or termination, clear the deferral and let the existing bounded `ExecCompletion` start one turn. Keep the default behavior unchanged for long-lived background services, and cover the Goal-plus-background-exec path with an integration test.
