// use std::collections::HashMap;
// use tokio::sync::mpsc::{unbounded, Receiver, Sender};

// pub struct Flow {
//     start_nodes: Vec<Sender<Msg>>,
// }

// pub struct Node {
//     inputs: Vec<Receiver<Msg>>,
//     outputs: Vec<Sender<Msg>>,
//     command: Box<dyn Cmd>,
// }

// // key: run_id|node_id

// type Msg = i32;

// const START_ENTRY_MARKER: &str = "START_ENTRY_MARKER";
// const FLOW_ENTRY_MARKER: &str = "FLOW_ENTRY_MARKER";

// #[tokio::test(flavor = "multi_thread")]
// async fn test_flow_ctx() {
//     let store = sunshine_indra::store::DB::new(&sunshine_indra::store::DbConfig {
//         db_path: "test_indra_db_flow_ctx".to_owned(),
//     })
//     .unwrap();

//     let store = Arc::new(store);

//     let flow_ctx = FlowContext::new(
//         Config {
//             url: "https://api.devnet.solana.com".into(),
//             keyring: HashMap::new(),
//             pub_keys: HashMap::new(),
//         },
//         store.clone(),
//     )
//     .unwrap();

//     let graph_id = store
//         .execute(Action::CreateGraph(Default::default()))
//         .await
//         .unwrap()
//         .as_id()
//         .unwrap();

//     // node 1
//     let mut props = serde_json::Map::new();

//     props.insert(START_COMMAND_MARKER.into(), JsonValue::Bool(true));
//     props.insert(
//         COMMAND_MARKER.into(),
//         serde_json::to_value(&Command::Print("hello1".into())).unwrap(),
//     );

//     let node1 = store
//         .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
//         .await
//         .unwrap()
//         .as_id()
//         .unwrap();

//     // node 2
//     let mut props = serde_json::Map::new();

//     props.insert(START_COMMAND_MARKER.into(), JsonValue::Bool(true));
//     props.insert(
//         COMMAND_MARKER.into(),
//         serde_json::to_value(&Command::Print("hello2".into())).unwrap(),
//     );

//     let node2 = store
//         .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
//         .await
//         .unwrap()
//         .as_id()
//         .unwrap();

//     // node 3
//     let mut props = serde_json::Map::new();

//     props.insert(START_COMMAND_MARKER.into(), JsonValue::Bool(true));
//     props.insert(
//         COMMAND_MARKER.into(),
//         serde_json::to_value(&Command::Print("hello3".into())).unwrap(),
//     );

//     let node3 = store
//         .execute(Action::Mutate(graph_id, MutateKind::CreateNode(props)))
//         .await
//         .unwrap()
//         .as_id()
//         .unwrap();

//     store
//         .execute(Action::Mutate(
//             graph_id,
//             MutateKind::CreateEdge(CreateEdge {
//                 from: node1,
//                 to: node2,
//                 properties: Default::default(),
//             }),
//         ))
//         .await
//         .unwrap();

//     store
//         .execute(Action::Mutate(
//             graph_id,
//             MutateKind::CreateEdge(CreateEdge {
//                 from: node1,
//                 to: node3,
//                 properties: Default::default(),
//             }),
//         ))
//         .await
//         .unwrap();

//     flow_ctx
//         .deploy_flow(Duration::from_secs(5), graph_id)
//         .await
//         .unwrap();

//     tokio::time::sleep(Duration::from_secs(11)).await;
// }
