use super::*;
use pretty_assertions::assert_eq;
use rmcp::model::ListResourcesResult;
use rmcp::model::Resource;

struct ResourceServer {
    name: &'static str,
    requests: Arc<AtomicUsize>,
}

impl ServerHandler for ResourceServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_resources().build())
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        self.requests.fetch_add(1, Ordering::SeqCst);
        Ok(ListResourcesResult::with_all_items(vec![Resource::new(
            format!("test://{}", self.name),
            self.name,
        )]))
    }
}

impl InProcessTransportFactory for ResourceServer {
    fn open(&self) -> BoxFuture<'static, io::Result<DuplexStream>> {
        let server = Self {
            name: self.name,
            requests: Arc::clone(&self.requests),
        };
        async move {
            let (client_stream, server_stream) = tokio::io::duplex(4096);
            tokio::spawn(async move {
                server
                    .serve(server_stream)
                    .await
                    .expect("serve resources")
                    .waiting()
                    .await
                    .expect("resource server completes");
            });
            Ok(client_stream)
        }
        .boxed()
    }
}

async fn resource_client(name: &'static str) -> (ManagedClient, Arc<AtomicUsize>) {
    let requests = Arc::new(AtomicUsize::new(0));
    let client = RmcpClient::new_in_process_client(Arc::new(ResourceServer {
        name,
        requests: Arc::clone(&requests),
    }))
    .await
    .expect("resource client");
    client
        .initialize(
            InitializeRequestParams::new(
                ClientCapabilities::default(),
                Implementation::new("codex-test", "0.0.0-test"),
            )
            .with_protocol_version(ProtocolVersion::V_2025_06_18),
            Some(Duration::from_secs(5)),
            Box::new(|_, _| async { Err(anyhow!("unexpected elicitation")) }.boxed()),
        )
        .await
        .expect("initialize resource client");
    let mut managed = create_test_managed_client(Vec::new()).await;
    managed.client = Arc::new(client);
    (managed, requests)
}

#[tokio::test(start_paused = true)]
async fn search_discovery_rejects_replaced_pending_resource_connection() -> anyhow::Result<()> {
    let home = tempdir()?;
    let environments = Arc::new(environment_manager_without_environments());
    environments.upsert_environment(
        "remote".to_string(),
        "ws://127.0.0.1:1".to_string(),
        /*connect_timeout*/ None,
    )?;
    let context = McpRuntimeContext::new(Arc::clone(&environments), PathBuf::from("/tmp"));
    let mut server = reusable_server_config("http://127.0.0.1:1/mcp");
    server.environment_id = "remote".to_string();
    let key = McpServerKey::unscoped("docs");
    let mut config = crate::mcp::tests::test_mcp_config(home.path().into());
    let mut catalog = crate::ResolvedMcpCatalog::builder();
    catalog.register(crate::McpServerRegistration::from_config(
        "docs".to_string(),
        server.clone(),
    ));
    config.mcp_server_catalog = catalog.build();
    config
        .server_permissions
        .insert(key.clone(), McpServerPermissions::Thread);
    config.optional_mcp_startup_grace = Duration::from_millis(1);

    let (old_client, old_requests) = resource_client("old").await;
    let (old_pending, old_started, release_old) = create_gated_async_managed_client(old_client);
    let mut previous = manager_with_reusable_ready_server(&server, &context, Vec::new()).await;
    Arc::get_mut(&mut previous.servers.get_mut(&key).unwrap().connection)
        .expect("one initial connection owner")
        .client = old_pending;
    let same = reconcile_reusable_server_with_mcp_config(
        &previous,
        "docs",
        server.clone(),
        context.clone(),
        config.clone(),
    )
    .await;
    assert!(previous.shares_test_connection_with(&same, "docs"));

    // Only the remote environment handle changes; registration and authority stay fixed.
    environments.upsert_environment(
        "remote".to_string(),
        "ws://127.0.0.1:2".to_string(),
        /*connect_timeout*/ None,
    )?;
    let mut replaced = reconcile_reusable_server_with_mcp_config(
        &previous,
        "docs",
        server,
        context,
        config.clone(),
    )
    .await;
    assert!(!previous.shares_test_connection_with(&replaced, "docs"));
    let (new_client, new_requests) = resource_client("new").await;
    let (new_pending, new_started, release_new) = create_gated_async_managed_client(new_client);
    // Gate startup at the managed-client boundary so neither capture can observe readiness.
    // The identity and connection allocation above still come from real reconciliation.
    let connection = Arc::get_mut(&mut replaced.servers.get_mut(&key).unwrap().connection)
        .expect("one replacement connection owner");
    connection.client.cancel_token.cancel();
    connection.client = new_pending;

    let config = Arc::new(config);
    let mut bindings = Vec::new();
    for connections in [previous, same, replaced] {
        let binding = Arc::new(connections)
            .capture_binding_with_metadata(
                Arc::clone(&config),
                /*plugins_available*/ false,
                /*required_servers*/ &[],
                /*required_plugins*/ &HashSet::new(),
            )
            .await;
        assert!(binding.tools().is_empty());
        assert!(binding.prepare_call(&key, "search").is_none());
        let listed = binding.list_all_resources(|_| true).await;
        assert_eq!(
            (listed.by_server, listed.deferred_servers),
            (HashMap::new(), Vec::new())
        );
        bindings.push(binding);
    }
    old_started.await?;
    new_started.await?;
    let [previous, same, replaced]: [McpBinding; 3] = bindings.try_into().ok().unwrap();
    let same_reusable = previous.has_same_search_discovery_inputs(&same);
    let replaced_reusable = previous.has_same_search_discovery_inputs(&replaced);
    let selected = if replaced_reusable {
        &previous
    } else {
        &replaced
    };
    release_old.send(()).unwrap();
    release_new.send(()).unwrap();
    let resources = selected.list_resources(&key, /*params*/ None).await?;
    let mut expected =
        ListResourcesResult::with_all_items(vec![Resource::new("test://new", "new")]);
    expected.result_type = None;
    assert_eq!(
        (
            same_reusable,
            replaced_reusable,
            resources,
            [
                old_requests.load(Ordering::SeqCst),
                new_requests.load(Ordering::SeqCst)
            ]
        ),
        (true, false, expected, [0, 1]),
    );
    Ok(())
}
