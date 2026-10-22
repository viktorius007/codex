use super::LocalAgentControl;
use crate::StartThreadOptions;
use crate::ThreadManager;
use crate::agent::types::AgentMetadata;
use crate::codex_thread::CodexThread;
use crate::config::Config;
use crate::config::test_config;
use crate::rollout::RolloutRecorder;
use crate::session::SessionSettingsUpdate;
use codex_features::Feature;
use codex_login::CodexAuth;
use codex_protocol::ThreadId;
use codex_protocol::protocol::EnvironmentConfigState;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::protocol::ThreadSource;
use codex_protocol::protocol::TurnEnvironmentSelections;
use std::sync::Arc;

struct Fixture {
    _home: tempfile::TempDir,
    config: Config,
    manager: ThreadManager,
    control: LocalAgentControl,
    root: Arc<CodexThread>,
    parent: Arc<CodexThread>,
    child: Arc<CodexThread>,
}

impl Fixture {
    async fn new() -> Self {
        let home = tempfile::tempdir().expect("temporary home");
        let mut config = test_config().await;
        let _ = config.features.enable(Feature::MultiAgentV2);
        config.multi_agent_v2.max_concurrent_threads_per_session = 8;
        config.permissions.allow_login_shell = true;
        config.codex_home = home.path().to_path_buf().try_into().unwrap();
        config.cwd = home.path().to_path_buf().try_into().unwrap();
        let manager = ThreadManager::with_models_provider_and_home_for_tests(
            CodexAuth::from_api_key("dummy"),
            config.model_provider.clone(),
            config.codex_home.to_path_buf(),
            Arc::new(codex_exec_server::EnvironmentManager::default_for_tests()),
        );
        let root = manager
            .start_thread(StartThreadOptions::new(config.clone()))
            .await
            .expect("start root")
            .thread;
        let control = root
            .session
            .services
            .local_agent_runtime
            .control(root.session.session_id());
        let parent = spawn_child(&control, config.clone(), root.session.thread_id).await;
        let child = spawn_child(&control, config.clone(), parent.session.thread_id).await;
        Self {
            _home: home,
            config,
            manager,
            control,
            root,
            parent,
            child,
        }
    }

    async fn pin(&self) -> super::ParentResultResidencyGuard {
        self.control
            .parent_result_residency(
                self.child.session.thread_id,
                MultiAgentVersion::V2,
                &self.child.session_source,
                &self.child.session,
            )
            .await
            .expect("admit child result parent")
            .expect("resident parent needs a guard")
    }
}

async fn spawn_child(
    control: &LocalAgentControl,
    config: Config,
    parent: ThreadId,
) -> Arc<CodexThread> {
    let state = control.runtime.upgrade().expect("manager alive");
    let child = state
        .spawn_child_thread(
            crate::thread_manager::StartThreadOptions {
                session_source: Some(SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
                    parent_thread_id: parent,
                    depth: 1,
                    agent_path: None,
                    agent_nickname: None,
                    agent_role: None,
                })),
                thread_source: Some(ThreadSource::Subagent),
                ..crate::thread_manager::StartThreadOptions::new(config)
            },
            control.clone(),
            Some(parent),
            /*inherited_exec_policy*/ None,
        )
        .await
        .expect("spawn child fixture");
    control
        .runtime
        .registry
        .reserve_spawn_slot(None)
        .expect("register fixture")
        .commit(AgentMetadata {
            agent_id: Some(child.thread_id),
            ..Default::default()
        });
    child.thread
}

#[tokio::test]
async fn dropping_result_guard_releases_parent_gate() {
    let fixture = Fixture::new().await;
    let guard = fixture.pin().await;
    assert!(
        Arc::clone(&fixture.parent.residency_gate)
            .try_write_owned()
            .is_err(),
        "admitted child result must protect its parent"
    );
    drop(guard);
    assert!(
        Arc::clone(&fixture.parent.residency_gate)
            .try_write_owned()
            .is_ok(),
        "completed result guard must release its parent"
    );
}

#[tokio::test]
async fn child_removal_releases_parent_pin_with_guard_and_child_retained() {
    let fixture = Fixture::new().await;
    let guard = fixture.pin().await;
    assert!(
        Arc::clone(&fixture.parent.residency_gate)
            .try_write_owned()
            .is_err(),
        "live result guard must pin the parent before removal"
    );
    let removed = fixture
        .manager
        .remove_thread(&fixture.child.session.thread_id)
        .await
        .expect("remove child");
    assert!(Arc::ptr_eq(&removed, &fixture.child));
    assert!(
        Arc::clone(&fixture.parent.residency_gate)
            .try_write_owned()
            .is_ok(),
        "explicit child removal must release the parent despite retained child and result guard"
    );
    drop((guard, removed));
}

#[tokio::test]
async fn rejected_delayed_removal_preserves_parent_pin() {
    let fixture = Fixture::new().await;
    let guard = fixture.pin().await;
    assert!(
        fixture
            .manager
            .remove_thread_if_matches(&fixture.child.session.thread_id, &fixture.root)
            .await
            .is_none()
    );
    assert!(
        Arc::clone(&fixture.parent.residency_gate)
            .try_write_owned()
            .is_err(),
        "rejected removal must preserve the registered child's parent pin"
    );
    fixture
        .manager
        .remove_thread_if_matches(&fixture.child.session.thread_id, &fixture.child)
        .await
        .expect("matching removal");
    assert!(
        Arc::clone(&fixture.parent.residency_gate)
            .try_write_owned()
            .is_ok(),
        "matching removal must cancel its child's parent pin"
    );
    drop(guard);
}

#[tokio::test]
async fn stopped_child_replacement_releases_old_parent_pin() {
    let fixture = Fixture::new().await;
    let guard = fixture.pin().await;
    fixture.child.ensure_rollout_materialized().await;
    let rollout_path = fixture.child.rollout_path().expect("child rollout path");
    fixture
        .child
        .flush_rollout()
        .await
        .expect("flush child rollout");
    let history = RolloutRecorder::get_rollout_history(&rollout_path)
        .await
        .expect("child rollout history");
    fixture
        .child
        .shutdown_and_wait()
        .await
        .expect("stop old child");
    assert!(
        Arc::clone(&fixture.parent.residency_gate)
            .try_write_owned()
            .is_err(),
        "retained result guard pins parent until old runtime is removed"
    );
    let replacement = fixture
        .manager
        .start_thread(StartThreadOptions {
            initial_history: history,
            session_source: Some(fixture.child.session_source.clone()),
            ..StartThreadOptions::new(fixture.config.clone())
        })
        .await
        .expect("replace stopped child");
    assert!(!Arc::ptr_eq(&replacement.thread, &fixture.child));
    assert!(
        Arc::clone(&fixture.parent.residency_gate)
            .try_write_owned()
            .is_ok(),
        "replacing stopped child must cancel old parent pin while old child and guard remain alive"
    );
    drop(guard);
}

#[tokio::test]
async fn result_admission_restores_parent_with_current_loaded_ancestor_authority() {
    let fixture = Fixture::new().await;
    fixture.parent.ensure_rollout_materialized().await;
    fixture
        .parent
        .shutdown_and_wait()
        .await
        .expect("stop parent");
    fixture
        .manager
        .remove_thread(&fixture.parent.session.thread_id)
        .await
        .expect("unload parent");
    let root_turn = fixture.root.session.new_default_turn().await;
    let root_environment = root_turn
        .initial_environments
        .primary()
        .expect("root environment");
    let mut owner_config = root_environment.config().clone();
    owner_config.allow_login_shell = false;
    let mut selection = root_environment.selection();
    selection.config = EnvironmentConfigState::Ready(owner_config);
    fixture
        .root
        .session
        .update_settings(SessionSettingsUpdate {
            environments: Some(TurnEnvironmentSelections::new(
                root_turn.config.cwd.clone(),
                vec![selection],
            )),
            ..Default::default()
        })
        .await
        .expect("change current root authority");
    assert!(
        fixture
            .manager
            .get_thread(fixture.parent.session.thread_id)
            .await
            .is_err(),
        "parent must be absent before admission restores it"
    );
    let guard = fixture.pin().await;
    let restored = fixture
        .manager
        .get_thread(fixture.parent.session.thread_id)
        .await
        .expect("restored parent");
    assert!(!Arc::ptr_eq(&restored, &fixture.parent));
    let restored_turn = restored.session.new_default_turn().await;
    assert!(
        !restored_turn
            .initial_environments
            .primary()
            .expect("restored environment")
            .config()
            .allow_login_shell,
        "restored parent must inherit current loaded ancestor authority rather than stale stored authority"
    );
    assert!(
        Arc::clone(&restored.residency_gate)
            .try_write_owned()
            .is_err(),
        "result admission must keep the restored direct parent pinned"
    );
    drop(guard);
    assert!(
        Arc::clone(&restored.residency_gate)
            .try_write_owned()
            .is_ok(),
        "completed child result releases restored parent"
    );
}
