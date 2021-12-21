use std::sync::Arc;
use std::time::Duration;

use serde_json::Value as JsonValue;
use sunshine_core::msg::{CreateEdge, MutateKind};
use sunshine_core::{msg::Action, store::Datastore};
use sunshine_indra::store::{DbConfig, DB};
use sunshine_solana::commands::simple;
use sunshine_solana::commands::solana::{self, Kind};
use sunshine_solana::{
    commands, FlowContext, COMMAND_MARKER, CTX_EDGE_MARKER, CTX_MARKER, INPUT_ARG_NAME_MARKER,
    OUTPUT_ARG_NAME_MARKER, START_NODE_MARKER,
};

#[tokio::main]
async fn main() {
    // create database
    let db_config = DbConfig {
        db_path: "flow_db".into(),
    };
    let db = DB::new(&db_config).unwrap();
    let db = Arc::new(db);

    // create wallet
    let wallet_graph_id = db
        .execute(Action::CreateGraph(Default::default()))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // create graph root
    let flow_graph_id = db
        .execute(Action::CreateGraph(Default::default()))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // create flow context
    let flow_context = FlowContext::new(db.clone());

    // create solana context node
    let mut props = serde_json::Map::new();

    let solana_context_config = solana::Config {
        url: "https://api.devnet.solana.com".into(),
        wallet_graph: wallet_graph_id,
    };

    props.insert(
        CTX_MARKER.into(),
        serde_json::to_value(&solana_context_config).unwrap(),
    );

    let solana_ctx_node_id = db
        .execute(Action::Mutate(flow_graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 0 - const seed
    let seed = "beach soldier piano click essay sock stable cover angle wear aunt advice";

    let simple_command = simple::Command::Const(sunshine_solana::Value::String(seed.into()));

    let mut props = serde_json::Map::new();

    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&commands::Config::Simple(simple_command)).unwrap(),
    );

    props.insert(START_NODE_MARKER.into(), JsonValue::Bool(true));

    let node0 = db
        .execute(Action::Mutate(flow_graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    //
    // node 1 - generate keypair

    let keypair = solana::generate_keypair::GenerateKeypair {
        seed_phrase: None,
        passphrase: Some("pass".into()),
        save: Some(Some("first_keypair".into())),
    };
    let mut props = serde_json::Map::new();

    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&commands::Config::Solana(Kind::GenerateKeypair(keypair))).unwrap(),
    );

    let node1 = db
        .execute(Action::Mutate(flow_graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 2 - keypair to pubkey

    let mut props = serde_json::Map::new();

    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&commands::Config::Simple(
            simple::Command::GetPubkeyFromKeypair,
        ))
        .unwrap(),
    );

    let node2 = db
        .execute(Action::Mutate(flow_graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 3 - print pubkey

    let mut props = serde_json::Map::new();

    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&commands::Config::Simple(simple::Command::Print)).unwrap(),
    );

    let node3 = db
        .execute(Action::Mutate(flow_graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // node 4 - airdrop
    let airdrop = solana::request_airdrop::RequestAirdrop {
        pubkey: None,
        amount: Some(1312313),
    };
    let mut props = serde_json::Map::new();

    props.insert(
        COMMAND_MARKER.into(),
        serde_json::to_value(&commands::Config::Solana(Kind::RequestAirdrop(airdrop))).unwrap(),
    );

    let node4 = db
        .execute(Action::Mutate(flow_graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // CONNECTIONS

    db.execute(Action::Mutate(
        flow_graph_id,
        MutateKind::CreateEdge(CreateEdge {
            from: node1,
            to: node4,
            properties: serde_json::json! ({
                INPUT_ARG_NAME_MARKER: "pubkey",
                OUTPUT_ARG_NAME_MARKER: "pubkey",
            })
            .as_object()
            .unwrap()
            .clone(),
        }),
    ))
    .await
    .unwrap();

    db.execute(Action::Mutate(
        flow_graph_id,
        MutateKind::CreateEdge(CreateEdge {
            from: node2,
            to: node3,
            properties: serde_json::json! ({
                INPUT_ARG_NAME_MARKER: "p",
                OUTPUT_ARG_NAME_MARKER: "pubkey",
            })
            .as_object()
            .unwrap()
            .clone(),
        }),
    ))
    .await
    .unwrap();

    db.execute(Action::Mutate(
        flow_graph_id,
        MutateKind::CreateEdge(CreateEdge {
            from: node1,
            to: node2,
            properties: serde_json::json! ({
                INPUT_ARG_NAME_MARKER: "keypair",
                OUTPUT_ARG_NAME_MARKER: "keypair",
            })
            .as_object()
            .unwrap()
            .clone(),
        }),
    ))
    .await
    .unwrap();

    db.execute(Action::Mutate(
        flow_graph_id,
        MutateKind::CreateEdge(CreateEdge {
            from: node0,
            to: node1,
            properties: serde_json::json! ({
                INPUT_ARG_NAME_MARKER: "seed_phrase",
                OUTPUT_ARG_NAME_MARKER: "res",
            })
            .as_object()
            .unwrap()
            .clone(),
        }),
    ))
    .await
    .unwrap();

    // // edge from wallet/solana context to node1
    db.execute(Action::Mutate(
        flow_graph_id,
        MutateKind::CreateEdge(CreateEdge {
            from: solana_ctx_node_id,
            to: node1,
            properties: serde_json::json! ({
                CTX_EDGE_MARKER: CTX_EDGE_MARKER,
            })
            .as_object()
            .unwrap()
            .clone(),
        }),
    ))
    .await
    .unwrap();

    db.execute(Action::Mutate(
        flow_graph_id,
        MutateKind::CreateEdge(CreateEdge {
            from: solana_ctx_node_id,
            to: node4,
            properties: serde_json::json! ({
                CTX_EDGE_MARKER: CTX_EDGE_MARKER,
            })
            .as_object()
            .unwrap()
            .clone(),
        }),
    ))
    .await
    .unwrap();

    // db.create_edge(
    //     CreateEdge {
    //         from: wallet1_graph_node_id,
    //         to: node1,
    //         properties: serde_json::json! ({
    //             "CTX_EDGE_MARKER": "CTX_EDGE_MARKER",
    //         })
    //         .as_object()
    //         .unwrap()
    //         .clone(),
    //     },
    //     db_graph_id,
    // );

    // deploy
    flow_context
        .deploy_flow(Duration::from_secs(3), flow_graph_id)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(7)).await;

    // create flow graph
    // create solana context nodes
    // add commands
    // connect commands

    // create flow context

    // deploy flow
}
