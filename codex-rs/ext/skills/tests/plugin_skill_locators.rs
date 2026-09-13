use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use codex_config::ConfigLayerEntry;
use codex_config::ConfigLayerSource;
use codex_config::ConfigLayerStack;
use codex_config::ConfigRequirementsToml;
use codex_extension_api::ConversationHistory;
use codex_extension_api::ExtensionData;
use codex_extension_api::ExtensionRegistry;
use codex_extension_api::ExtensionRegistryBuilder;
use codex_extension_api::FunctionCallError;
use codex_extension_api::NoopTurnItemEmitter;
use codex_extension_api::PreviousWorldStateSection;
use codex_extension_api::ThreadStartInput;
use codex_extension_api::ToolCall;
use codex_extension_api::ToolCallSource;
use codex_extension_api::ToolPayload;
use codex_extension_api::TurnInputContext;
use codex_extension_api::WorldStateContributionInput;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::TruncationPolicy;
use codex_protocol::user_input::UserInput;
use codex_skills_extension::HostSkillsLoadInput;
use codex_skills_extension::HostSkillsService;
use codex_skills_extension::HostSkillsSnapshot;
use codex_skills_extension::SkillsExtensionConfig;
use codex_skills_extension::install;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_plugins::PluginIdentity;
use codex_utils_plugins::PluginSkillRoot;
use codex_utils_plugins::SkillDiscoveryMode;
use pretty_assertions::assert_eq;
use serde_json::Value;
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const PRIMARY_PLUGIN_ID: &str = "shared@marketplace-one";
const PRIMARY_PLUGIN_PACKAGE: &str = "skill://shared@marketplace-one/analyze";
const PRIMARY_PLUGIN_SKILL: &str =
    "---\nname: analyze\ndescription: Analyze data.\n---\n\n# Primary analyzer\n";
const PRIMARY_REFERENCE_A: &str = "reference from revision A\n";
const PRIMARY_REFERENCE_B: &str = "reference from revision B\n";
const COLLIDING_PLUGIN_ID: &str = "shared@marketplace-two";
const COLLIDING_PLUGIN_PACKAGE: &str = "skill://shared@marketplace-two/analyze";
const COLLIDING_PLUGIN_SKILL: &str =
    "---\nname: analyze\ndescription: Analyze data.\n---\n\n# Marketplace two analyzer\n";
const LOCAL_SKILL: &str =
    "---\nname: local-check\ndescription: Check local files.\n---\n\n# Local check\n";

#[derive(Clone)]
struct TestConfig;

struct PluginFixture {
    skill_root: PluginSkillRoot,
    revision_root: PathBuf,
}

#[derive(Clone, Copy)]
enum ReadTarget<'a> {
    Main,
    Resource(&'a str),
}

fn skills_extension_config(_: &TestConfig) -> SkillsExtensionConfig {
    SkillsExtensionConfig {
        include_instructions: true,
        max_context_tokens: None,
        bundled_skills_enabled: false,
        orchestrator_skills_enabled: false,
        shadow_selection_enabled: false,
    }
}

fn config_stack() -> TestResult<ConfigLayerStack> {
    Ok(ConfigLayerStack::new(
        vec![ConfigLayerEntry::new(
            ConfigLayerSource::SessionFlags,
            toml::from_str("[skills.bundled]\nenabled = false\n")?,
        )],
        Default::default(),
        ConfigRequirementsToml::default(),
    )?)
}

fn write_plugin_skill(
    codex_home: &Path,
    marketplace: &str,
    revision: &str,
    plugin_id: &str,
    contents: &str,
    reference_contents: &str,
) -> TestResult<PluginFixture> {
    let revision_root = codex_home
        .join("plugins/cache")
        .join(marketplace)
        .join("shared")
        .join(revision);
    let skill_root = revision_root.join("skills");
    let skill_dir = skill_root.join("analyze");
    std::fs::create_dir_all(revision_root.join(".codex-plugin"))?;
    std::fs::create_dir_all(skill_dir.join("references"))?;
    std::fs::write(
        revision_root.join(".codex-plugin/plugin.json"),
        r#"{"name":"shared"}"#,
    )?;
    std::fs::write(skill_dir.join("SKILL.md"), contents)?;
    std::fs::write(skill_dir.join("references/x.md"), reference_contents)?;
    let revision_root = std::fs::canonicalize(revision_root)?;
    let skill_root = std::fs::canonicalize(skill_root)?;

    Ok(PluginFixture {
        skill_root: PluginSkillRoot {
            path: AbsolutePathBuf::try_from(skill_root)?,
            plugin_identity: PluginIdentity {
                plugin_id: plugin_id.to_string(),
                remote_plugin_id: None,
            },
            plugin_namespace: "shared".to_string(),
            plugin_root: AbsolutePathBuf::try_from(revision_root.clone())?,
            discovery_mode: SkillDiscoveryMode::Recursive,
        },
        revision_root,
    })
}

async fn start_registry()
-> TestResult<(ExtensionRegistry<TestConfig>, ExtensionData, ExtensionData)> {
    let mut builder = ExtensionRegistryBuilder::new();
    install(&mut builder, skills_extension_config);
    let registry = builder.build();
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new("thread");
    registry.thread_lifecycle_contributors()[0]
        .on_thread_start(ThreadStartInput {
            config: &TestConfig,
            session_source: &SessionSource::Cli,
            persistent_thread_state_available: true,
            environments: &[],
            mcp_resource_client: None,
            extension_metrics: None,
            session_store: &session_store,
            thread_store: &thread_store,
        })
        .await;
    Ok((registry, session_store, thread_store))
}

async fn load_snapshot(
    service: &HostSkillsService,
    cwd: &Path,
    plugin_roots: Vec<PluginSkillRoot>,
) -> TestResult<HostSkillsSnapshot> {
    Ok(service
        .snapshot_for_config(
            &HostSkillsLoadInput::new(
                AbsolutePathBuf::try_from(cwd.to_path_buf())?,
                plugin_roots,
                config_stack()?,
            ),
            /*fs*/ None,
        )
        .await)
}

async fn render_host_catalog(
    registry: &ExtensionRegistry<TestConfig>,
    session_store: &ExtensionData,
    thread_store: &ExtensionData,
    turn_id: &str,
    snapshot: HostSkillsSnapshot,
) -> TestResult<(ExtensionData, String)> {
    let turn_store = ExtensionData::new(turn_id);
    turn_store.insert(snapshot);
    let sections = registry.context_contributors()[0]
        .contribute_world_state(WorldStateContributionInput {
            thread_id: codex_protocol::ThreadId::new(),
            turn_id,
            environments: &[],
            ready_selected_capability_roots: &[],
            executor_capability_discovery: None,
            extension_metrics: None,
            session_store,
            thread_store,
            turn_store: &turn_store,
        })
        .await;
    let catalog = sections
        .iter()
        .find(|section| section.id() == "host_skills")
        .ok_or("host catalog section should exist")?
        .render_diff(PreviousWorldStateSection::Absent)
        .ok_or("host catalog should render")?
        .body()
        .to_string();
    Ok((turn_store, catalog))
}

async fn read_package(
    registry: &ExtensionRegistry<TestConfig>,
    session_store: &ExtensionData,
    thread_store: &ExtensionData,
    turn_store: &ExtensionData,
    package: &str,
    target: ReadTarget<'_>,
) -> Result<Value, FunctionCallError> {
    let tools =
        registry.tool_contributors()[0].tools_for_step(session_store, thread_store, turn_store);
    let read_tool = tools
        .iter()
        .find(|tool| tool.tool_name().name == "read")
        .ok_or_else(|| {
            FunctionCallError::Fatal(
                "skills.read should be registered for plugin-backed host skills".to_string(),
            )
        })?;
    let arguments = match target {
        ReadTarget::Main => serde_json::json!({"package": package}),
        ReadTarget::Resource(resource) => {
            serde_json::json!({"package": package, "resource": resource})
        }
    };
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let call_id = format!("read-{package}");
    let output = read_tool
        .handle(ToolCall {
            turn_id: turn_store.level_id().to_string(),
            call_id: call_id.clone(),
            tool_name: read_tool.tool_name(),
            model: "gpt-test".to_string(),
            codex_turn_metadata: None,
            truncation_policy: TruncationPolicy::Bytes(16_000),
            source: ToolCallSource::Direct,
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            environments: Vec::new(),
            payload: payload.clone(),
        })
        .await?;
    output
        .post_tool_use_response(&call_id, &payload)
        .ok_or_else(|| {
            FunctionCallError::Fatal("skills.read should expose structured output".to_string())
        })
}

#[tokio::test]
async fn plugin_packages_stay_stable_across_cache_revisions_and_resolve_the_active_snapshot()
-> TestResult {
    let codex_home = TempDir::new()?;
    let cwd = codex_home.path().join("workspace");
    std::fs::create_dir_all(&cwd)?;
    let revision_a = write_plugin_skill(
        codex_home.path(),
        "marketplace-one",
        "revision-a",
        PRIMARY_PLUGIN_ID,
        PRIMARY_PLUGIN_SKILL,
        PRIMARY_REFERENCE_A,
    )?;
    let revision_b = write_plugin_skill(
        codex_home.path(),
        "marketplace-one",
        "revision-b",
        PRIMARY_PLUGIN_ID,
        PRIMARY_PLUGIN_SKILL,
        PRIMARY_REFERENCE_B,
    )?;
    let colliding_plugin = write_plugin_skill(
        codex_home.path(),
        "marketplace-two",
        "revision-c",
        COLLIDING_PLUGIN_ID,
        COLLIDING_PLUGIN_SKILL,
        "marketplace two reference\n",
    )?;
    let service = HostSkillsService::new_with_restriction_product(
        AbsolutePathBuf::try_from(codex_home.path().to_path_buf())?,
        /*bundled_skills_enabled*/ false,
        /*restriction_product*/ None,
    );
    let snapshot_a = load_snapshot(
        &service,
        &cwd,
        vec![
            revision_a.skill_root.clone(),
            colliding_plugin.skill_root.clone(),
        ],
    )
    .await?;
    let snapshot_b = load_snapshot(
        &service,
        &cwd,
        vec![
            revision_b.skill_root.clone(),
            colliding_plugin.skill_root.clone(),
        ],
    )
    .await?;
    let (registry, session_store, thread_store) = start_registry().await?;
    let (_turn_a, catalog_a) = render_host_catalog(
        &registry,
        &session_store,
        &thread_store,
        "turn-a",
        snapshot_a,
    )
    .await?;
    let (turn_b, catalog_b) = render_host_catalog(
        &registry,
        &session_store,
        &thread_store,
        "turn-b",
        snapshot_b,
    )
    .await?;

    assert_eq!(catalog_a.as_bytes(), catalog_b.as_bytes());
    assert_eq!(
        catalog_b
            .lines()
            .filter(|line| line.starts_with("- shared:analyze:"))
            .map(str::to_string)
            .collect::<Vec<_>>(),
        vec![
            format!("- shared:analyze: Analyze data. (plugin package: {PRIMARY_PLUGIN_PACKAGE})"),
            format!("- shared:analyze: Analyze data. (plugin package: {COLLIDING_PLUGIN_PACKAGE})"),
        ]
    );
    for cache_revision in ["revision-a", "revision-b", "revision-c"] {
        assert!(!catalog_b.contains(cache_revision), "{catalog_b}");
    }

    std::fs::remove_dir_all(&revision_a.revision_root)?;
    assert_eq!(
        read_package(
            &registry,
            &session_store,
            &thread_store,
            &turn_b,
            PRIMARY_PLUGIN_PACKAGE,
            ReadTarget::Main,
        )
        .await?,
        serde_json::json!({
            "resource": format!("{PRIMARY_PLUGIN_PACKAGE}/SKILL.md"),
            "contents": PRIMARY_PLUGIN_SKILL,
            "next_cursor": null,
        })
    );
    assert_eq!(
        read_package(
            &registry,
            &session_store,
            &thread_store,
            &turn_b,
            COLLIDING_PLUGIN_PACKAGE,
            ReadTarget::Main,
        )
        .await?,
        serde_json::json!({
            "resource": format!("{COLLIDING_PLUGIN_PACKAGE}/SKILL.md"),
            "contents": COLLIDING_PLUGIN_SKILL,
            "next_cursor": null,
        })
    );

    Ok(())
}

#[tokio::test]
async fn plugin_package_subresources_follow_the_active_snapshot_and_stay_within_the_package()
-> TestResult {
    let codex_home = TempDir::new()?;
    let cwd = codex_home.path().join("workspace");
    std::fs::create_dir_all(&cwd)?;
    let revision_a = write_plugin_skill(
        codex_home.path(),
        "marketplace-one",
        "revision-a",
        PRIMARY_PLUGIN_ID,
        PRIMARY_PLUGIN_SKILL,
        PRIMARY_REFERENCE_A,
    )?;
    let revision_b = write_plugin_skill(
        codex_home.path(),
        "marketplace-one",
        "revision-b",
        PRIMARY_PLUGIN_ID,
        PRIMARY_PLUGIN_SKILL,
        PRIMARY_REFERENCE_B,
    )?;
    let outside_package = revision_b.skill_root.path.join("outside.md");
    std::fs::write(&outside_package, "outside the skill package\n")?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        &outside_package,
        revision_b
            .skill_root
            .path
            .join("analyze/references/escape.md"),
    )?;

    let service = HostSkillsService::new_with_restriction_product(
        AbsolutePathBuf::try_from(codex_home.path().to_path_buf())?,
        /*bundled_skills_enabled*/ false,
        /*restriction_product*/ None,
    );
    let snapshot_a = load_snapshot(&service, &cwd, vec![revision_a.skill_root.clone()]).await?;
    let snapshot_b = load_snapshot(&service, &cwd, vec![revision_b.skill_root.clone()]).await?;
    let (registry, session_store, thread_store) = start_registry().await?;
    let (turn_a, catalog_a) = render_host_catalog(
        &registry,
        &session_store,
        &thread_store,
        "reference-turn-a",
        snapshot_a,
    )
    .await?;
    let (turn_b, catalog_b) = render_host_catalog(
        &registry,
        &session_store,
        &thread_store,
        "reference-turn-b",
        snapshot_b,
    )
    .await?;
    let reference = format!("{PRIMARY_PLUGIN_PACKAGE}/references/x.md");

    assert!(catalog_a.contains(PRIMARY_PLUGIN_PACKAGE));
    assert!(catalog_b.contains(PRIMARY_PLUGIN_PACKAGE));
    assert_eq!(
        read_package(
            &registry,
            &session_store,
            &thread_store,
            &turn_a,
            PRIMARY_PLUGIN_PACKAGE,
            ReadTarget::Resource(&reference),
        )
        .await?,
        serde_json::json!({
            "resource": reference,
            "contents": PRIMARY_REFERENCE_A,
            "next_cursor": null,
        })
    );

    std::fs::remove_dir_all(&revision_a.revision_root)?;
    assert_eq!(
        read_package(
            &registry,
            &session_store,
            &thread_store,
            &turn_b,
            PRIMARY_PLUGIN_PACKAGE,
            ReadTarget::Resource(&reference),
        )
        .await?,
        serde_json::json!({
            "resource": reference,
            "contents": PRIMARY_REFERENCE_B,
            "next_cursor": null,
        })
    );

    let traversal = format!("{PRIMARY_PLUGIN_PACKAGE}/references/../../outside.md");
    assert_eq!(
        read_package(
            &registry,
            &session_store,
            &thread_store,
            &turn_b,
            PRIMARY_PLUGIN_PACKAGE,
            ReadTarget::Resource(&traversal),
        )
        .await,
        Err(FunctionCallError::RespondToModel(
            "failed to read skill resource".to_string()
        ))
    );
    #[cfg(unix)]
    {
        let symlink_escape = format!("{PRIMARY_PLUGIN_PACKAGE}/references/escape.md");
        assert_eq!(
            read_package(
                &registry,
                &session_store,
                &thread_store,
                &turn_b,
                PRIMARY_PLUGIN_PACKAGE,
                ReadTarget::Resource(&symlink_escape),
            )
            .await,
            Err(FunctionCallError::RespondToModel(
                "failed to read skill resource".to_string()
            ))
        );
    }

    Ok(())
}

#[tokio::test]
async fn ordinary_host_skills_keep_filesystem_locators_and_direct_prompt_reads() -> TestResult {
    let codex_home = TempDir::new()?;
    let cwd = codex_home.path().join("workspace");
    let skill_path = codex_home.path().join("skills/local-check/SKILL.md");
    std::fs::create_dir_all(&cwd)?;
    std::fs::create_dir_all(
        skill_path
            .parent()
            .ok_or("local skill path should have a parent")?,
    )?;
    std::fs::write(&skill_path, LOCAL_SKILL)?;
    let service = HostSkillsService::new_with_restriction_product(
        AbsolutePathBuf::try_from(codex_home.path().to_path_buf())?,
        /*bundled_skills_enabled*/ false,
        /*restriction_product*/ None,
    );
    let skills_root = std::fs::canonicalize(
        skill_path
            .parent()
            .and_then(Path::parent)
            .ok_or("local skill path should be below a skills root")?,
    )?;
    service.set_extra_roots(vec![AbsolutePathBuf::try_from(skills_root.clone())?]);
    let snapshot = load_snapshot(&service, &cwd, Vec::new()).await?;
    let (registry, session_store, thread_store) = start_registry().await?;
    let turn_store = ExtensionData::new("local-turn");
    turn_store.insert(snapshot);

    let fragments = registry.turn_input_contributors()[0]
        .contribute(
            TurnInputContext {
                turn_id: "local-turn".to_string(),
                user_input: vec![UserInput::Text {
                    text: "$local-check".to_string(),
                    text_elements: Vec::new(),
                }],
                environments: Vec::new(),
            },
            /*extension_metrics*/ None,
            &session_store,
            &thread_store,
            &turn_store,
        )
        .await;
    let rendered_path = std::fs::canonicalize(skill_path)?
        .to_string_lossy()
        .replace('\\', "/");
    let rendered_root = skills_root.to_string_lossy().replace('\\', "/");

    assert_eq!(
        fragments
            .iter()
            .map(|fragment| fragment.role())
            .collect::<Vec<_>>(),
        vec!["developer", "user"]
    );
    assert_eq!(
        fragments[0]
            .render()
            .lines()
            .filter(|line| line.starts_with("- `r"))
            .map(str::to_string)
            .collect::<Vec<_>>(),
        vec![format!("- `r0` = `{rendered_root}`")]
    );
    assert_eq!(
        fragments[0]
            .render()
            .lines()
            .filter(|line| line.starts_with("- local-check:"))
            .map(str::to_string)
            .collect::<Vec<_>>(),
        vec!["- local-check: Check local files. (file: r0/local-check/SKILL.md)".to_string()]
    );
    assert_eq!(
        fragments[1].render(),
        format!(
            "<skill>\n<name>local-check</name>\n<path>{rendered_path}</path>\n{LOCAL_SKILL}\n</skill>"
        )
    );

    Ok(())
}
