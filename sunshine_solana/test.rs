
#[tokio::test(flavor = "multi_thread")]
async fn test_flow_ctx() {
    let store = sunshine_indra::store::DB::new(&sunshine_indra::store::DbConfig {
        db_path: "test_indra_db_flow_ctx".to_owned(),
    })
    .unwrap();

    let store = Arc::new(store);

    let flow_ctx = FlowContext::new(
        Config {
            url: "https://api.devnet.solana.com".into(),
            keyring: HashMap::new(),
            pub_keys: HashMap::new(),
        },
        store.clone(),
    )
    .unwrap();

    let graph_id = store
        .execute(Action::CreateGraph(Default::default()))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 1
    let mut props = serde_json::Map::new();

    props.insert(START_NODE_MARKER.into(), JsonValue::Bool(true));
    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&Command::Simple(SimpleCommand::Const(3))).unwrap(),
    );

    let node1 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 2
    let mut props = serde_json::Map::new();

    props.insert(START_NODE_MARKER.into(), JsonValue::Bool(true));
    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&SimpleCommand::Const(2)).unwrap(),
    );

    let node2 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 3
    let mut props = serde_json::Map::new();

    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&SimpleCommand::Add).unwrap(),
    );

    let node3 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 4
    let mut props = serde_json::Map::new();

    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&SimpleCommand::Print).unwrap(),
    );

    let node4 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 5
    let mut props = serde_json::Map::new();

    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&SimpleCommand::Const(7)).unwrap(),
    );

    let node5 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 6
    let mut props = serde_json::Map::new();

    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&SimpleCommand::Add).unwrap(),
    );

    let node6 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 7
    let mut props = serde_json::Map::new();

    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&SimpleCommand::Print).unwrap(),
    );

    let node7 = store
        .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();
    //
    store
        .execute(Action::Mutate(
            graph_id,
            MutateKind::CreateEdge(CreateEdge {
                from: node1,
                to: node3,
                properties: serde_json::json! ({
                    INPUT_ARG_NAME_MARKER: "a",
                    OUTPUT_ARG_NAME_MARKER: "res",
                })
                .as_object()
                .unwrap()
                .clone(),
            }),
        ))
        .await
        .unwrap();

    store
        .execute(Action::Mutate(
            graph_id,
            MutateKind::CreateEdge(CreateEdge {
                from: node2,
                to: node3,
                properties: serde_json::json!({
                    INPUT_ARG_NAME_MARKER: "b",
                    OUTPUT_ARG_NAME_MARKER: "res",
                })
                .as_object()
                .unwrap()
                .clone(),
            }),
        ))
        .await
        .unwrap();

    store
        .execute(Action::Mutate(
            graph_id,
            MutateKind::CreateEdge(CreateEdge {
                from: node3,
                to: node4,
                properties: serde_json::json!({
                    INPUT_ARG_NAME_MARKER: "p",
                    OUTPUT_ARG_NAME_MARKER: "res",
                })
                .as_object()
                .unwrap()
                .clone(),
            }),
        ))
        .await
        .unwrap();

    store
        .execute(Action::Mutate(
            graph_id,
            MutateKind::CreateEdge(CreateEdge {
                from: node3,
                to: node6,
                properties: serde_json::json!({
                    INPUT_ARG_NAME_MARKER: "a",
                    OUTPUT_ARG_NAME_MARKER: "res",
                })
                .as_object()
                .unwrap()
                .clone(),
            }),
        ))
        .await
        .unwrap();

    store
        .execute(Action::Mutate(
            graph_id,
            MutateKind::CreateEdge(CreateEdge {
                from: node5,
                to: node6,
                properties: serde_json::json!({
                    INPUT_ARG_NAME_MARKER: "b",
                    OUTPUT_ARG_NAME_MARKER: "res",
                })
                .as_object()
                .unwrap()
                .clone(),
            }),
        ))
        .await
        .unwrap();

    store
        .execute(Action::Mutate(
            graph_id,
            MutateKind::CreateEdge(CreateEdge {
                from: node6,
                to: node7,
                properties: serde_json::json!({
                    INPUT_ARG_NAME_MARKER: "p",
                    OUTPUT_ARG_NAME_MARKER: "res",
                })
                .as_object()
                .unwrap()
                .clone(),
            }),
        ))
        .await
        .unwrap();

    flow_ctx
        .deploy_flow(Duration::from_secs(5), graph_id)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(11)).await;
}

