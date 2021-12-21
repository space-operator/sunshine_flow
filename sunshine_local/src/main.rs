use std::sync::Arc;
use std::time::Duration;

use sunshine_core::msg::{CreateEdge, MutateKind};
use sunshine_core::{msg::Action, store::Datastore};
use sunshine_indra::store::{DbConfig, DB};
use sunshine_solana::commands::solana::{self, Kind};
use sunshine_solana::{commands, FlowContext};

#[tokio::main]
async fn main() {
    // create database
    let db_config = DbConfig {
        db_path: "flow_db".into(),
    };
    let db = DB::new(&db_config).unwrap();
    let db = Arc::new(db);

    // create graph root
    let db_graph_id = db
        .clone()
        .execute(Action::CreateGraph(Default::default()))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // create flow context
    let flow_context = FlowContext::new(db.clone());

    // create solana context node
    let mut props = serde_json::Map::new();

    let wallet1_graph_node_id = db
        .execute(Action::Mutate(db_graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    let solana_context_config = solana::Config {
        url: "https://api.devnet.solana.com".into(),
        wallet_graph: wallet1_graph_node_id.clone(),
    };

    // update context node with config properties
    let mut props = serde_json::Map::new();

    props.insert(
        "CTX_MARKER".into(),
        serde_json::to_value(&solana_context_config).unwrap(),
    );
    db.execute(Action::Mutate(
        db_graph_id,
        MutateKind::UpdateNode((wallet1_graph_node_id, props)),
    ));

    //
    // node 1 - generate keypair
    let seed = "beach soldier piano click essay sock stable cover angle wear aunt advice";

    let keypair = solana::generate_keypair::GenerateKeypair {
        seed_phrase: Some(seed.to_string()),
        passphrase: Some("pass".into()),
        save: Some(Some("first_keypair".into())),
    };
    let mut props = serde_json::Map::new();

    props.insert(
        "COMMAND_MARKER".into(),
        serde_json::to_value(&commands::Config::Solana(Kind::GenerateKeypair(keypair))).unwrap(),
    );

    let node1 = db
        .execute(Action::Mutate(db_graph_id, MutateKind::CreateNode(props)))
        .await
        .unwrap()
        .as_id()
        .unwrap();

    // edge from wallet/solana context to node1
    db.execute(Action::Mutate(
        db_graph_id,
        MutateKind::CreateEdge(CreateEdge {
            from: wallet1_graph_node_id,
            to: node1,
            properties: serde_json::json! ({
                "CTX_EDGE_MARKER": "CTX_EDGE_MARKER",
            })
            .as_object()
            .unwrap()
            .clone(),
        }),
    ))
    .await
    .unwrap();

    // deploy
    flow_context
        .deploy_flow(Duration::from_secs(3), db_graph_id)
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
