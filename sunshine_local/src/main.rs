use std::sync::Arc;
use std::time::Duration;

use serde_json::Value as JsonValue;
use sunshine_core::msg::{CreateEdge, MutateKind, NodeId};
use sunshine_core::{msg::Action, store::Datastore};
use sunshine_indra::store::{DbConfig, DB};
use sunshine_solana::commands::simple;
use sunshine_solana::commands::solana::transfer::Transfer;
use sunshine_solana::commands::solana::{self, Kind};
use sunshine_solana::{
    commands, FlowContext, COMMAND_MARKER, CTX_EDGE_MARKER, CTX_MARKER, INPUT_ARG_NAME_MARKER,
    OUTPUT_ARG_NAME_MARKER, START_NODE_MARKER,
};

use crate::solana::create_account::CreateAccount;
use crate::solana::create_token::CreateToken;
use crate::solana::generate_keypair::GenerateKeypair;
use crate::solana::mint_token::MintToken;

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
            from: node0,
            to: node1,
            properties: serde_json::json! ({
                OUTPUT_ARG_NAME_MARKER: "res",
                INPUT_ARG_NAME_MARKER: "seed_phrase",
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
                OUTPUT_ARG_NAME_MARKER: "keypair",
                INPUT_ARG_NAME_MARKER: "keypair",
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
                OUTPUT_ARG_NAME_MARKER: "pubkey",
                INPUT_ARG_NAME_MARKER: "print",
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
            to: node4,
            properties: serde_json::json! ({
                OUTPUT_ARG_NAME_MARKER: "pubkey",
                INPUT_ARG_NAME_MARKER: "pubkey",
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

    let add_node = |db: Arc<DB>,
                    cfg: commands::Config,
                    is_start_node: bool,
                    inbound_edges: Vec<(NodeId, JsonValue)>| async move {
        let mut props = serde_json::Map::new();

        props.insert(COMMAND_MARKER.into(), serde_json::to_value(cfg).unwrap());

        if is_start_node {
            props.insert(START_NODE_MARKER.into(), JsonValue::Bool(true));
        }

        let node_id = db
            .execute(Action::Mutate(flow_graph_id, MutateKind::CreateNode(props)))
            .await
            .unwrap()
            .as_id()
            .unwrap();

        for (from, props) in inbound_edges {
            db.execute(Action::Mutate(
                flow_graph_id,
                MutateKind::CreateEdge(CreateEdge {
                    from,
                    to: node_id,
                    properties: props.as_object().unwrap().clone(),
                }),
            ))
            .await
            .unwrap();
        }

        node_id
    };

    // used //kiss february ivory merge topic uncover female cancel innocent leg surprise cabbage
    // laugh toy good ring measure position random squirrel penalty prosper write liar
    // must motor sail initial budget moral drip asthma slide steak since lesson
    // used // hello deer force person lunch wonder cash crater happy security punch decade

    let node14 = add_node(
        db.clone(),
        commands::Config::Solana(Kind::GenerateKeypair(GenerateKeypair {
            seed_phrase: Some(
                "antenna ceiling age disagree obvious road true inform gun vintage mixed cereal"
                    .into(),
            ),
            passphrase: Some("asdasdas".into()),
            save: Some(Some("14".into())),
        })),
        true,
        vec![(
            solana_ctx_node_id,
            serde_json::json!({ CTX_EDGE_MARKER: CTX_EDGE_MARKER }),
        )],
    )
    .await;

    let node5 = add_node(
        db.clone(),
        commands::Config::Solana(Kind::CreateToken(CreateToken {
            fee_payer: None,
            decimals: Some(4),
            authority: None,
            token: None,
            memo: Some("SUNSHINE MINTING ACCOUNT 30000".into()),
        })),
        false,
        vec![
            (
                solana_ctx_node_id,
                serde_json::json!({ CTX_EDGE_MARKER: CTX_EDGE_MARKER }),
            ),
            (
                node1,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "fee_payer",
                }),
            ),
            (
                node1,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "authority",
                }),
            ),
            (
                node14,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "token",
                }),
            ),
        ],
    )
    .await;

    let node17 = add_node(
        db.clone(),
        commands::Config::Solana(Kind::GenerateKeypair(GenerateKeypair {
            seed_phrase: Some(
                "kiss february ivory merge topic uncover female cancel innocent leg surprise cabbage".into(),
            ),
            passphrase: Some("123123".into()),
            save: Some(Some("17".into())),
        })),
        true,
        vec![
            (
                solana_ctx_node_id,
                serde_json::json!({ CTX_EDGE_MARKER: CTX_EDGE_MARKER }),
            ),],
    )
    .await;

    let node6 = add_node(
        db.clone(),
        commands::Config::Solana(Kind::CreateAccount(CreateAccount {
            owner: None,
            fee_payer: None,
            token: None,
            account: None,
        })),
        false,
        vec![
            (
                solana_ctx_node_id,
                serde_json::json!({ CTX_EDGE_MARKER: CTX_EDGE_MARKER }),
            ),
            (
                node17,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "account",
                }),
            ),
            (
                node1,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "owner",
                }),
            ),
            (
                node1,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "fee_payer",
                }),
            ),
            (
                node5,
                serde_json::json!({
                    INPUT_ARG_NAME_MARKER: "token",
                    OUTPUT_ARG_NAME_MARKER: "token",
                }),
            ),
        ],
    )
    .await;

    let node8 = add_node(
        db.clone(),
        commands::Config::Simple(simple::Command::Print),
        false,
        vec![
            (
                solana_ctx_node_id,
                serde_json::json!({ CTX_EDGE_MARKER: CTX_EDGE_MARKER }),
            ),
            (
                node17,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "pubkey",
                    INPUT_ARG_NAME_MARKER: "print",
                }),
            ),
        ],
    )
    .await;

    let node15 = add_node(
        db.clone(),
        commands::Config::Simple(simple::Command::Print),
        false,
        vec![
            (
                solana_ctx_node_id,
                serde_json::json!({ CTX_EDGE_MARKER: CTX_EDGE_MARKER }),
            ),
            (
                node5,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "pubkey",
                    INPUT_ARG_NAME_MARKER: "print",
                }),
            ),
        ],
    )
    .await;

    let node9 = add_node(
        db.clone(),
        commands::Config::Solana(Kind::MintToken(MintToken {
            token: None,
            recipient: None,
            mint_authority: None,
            amount: Some(1.23456),
            fee_payer: None,
        })),
        false,
        vec![
            (
                solana_ctx_node_id,
                serde_json::json!({ CTX_EDGE_MARKER: CTX_EDGE_MARKER }),
            ),
            (
                node6,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "pubkey",
                    INPUT_ARG_NAME_MARKER: "recipient",
                }),
            ),
            (
                node6,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "pubkey",
                    INPUT_ARG_NAME_MARKER: "mint_authority",
                }),
            ),
            (
                node6,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "pubkey",
                    INPUT_ARG_NAME_MARKER: "fee_payer",
                }),
            ),
            (
                node5,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "token",
                    INPUT_ARG_NAME_MARKER: "token",
                }),
            ),
        ],
    )
    .await;

    let node18 = add_node(
        db.clone(),
        commands::Config::Solana(Kind::GenerateKeypair(GenerateKeypair {
            seed_phrase: Some(
                "hello deer force person lunch wonder cash crater happy security punch decade"
                    .into(),
            ),
            passphrase: Some("pass".into()),
            save: Some(Some("18".into())),
        })),
        true,
        vec![(
            solana_ctx_node_id,
            serde_json::json!({ CTX_EDGE_MARKER: CTX_EDGE_MARKER }),
        )],
    )
    .await;

    let node10 = add_node(
        db.clone(),
        commands::Config::Simple(simple::Command::Print),
        false,
        vec![
            (
                solana_ctx_node_id,
                serde_json::json!({ CTX_EDGE_MARKER: CTX_EDGE_MARKER }),
            ),
            (
                node17,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "pubkey",
                    INPUT_ARG_NAME_MARKER: "print",
                }),
            ),
        ],
    )
    .await;

    let node7 = add_node(
        db.clone(),
        commands::Config::Solana(Kind::CreateAccount(CreateAccount {
            owner: None,
            fee_payer: None,
            token: None,
            account: None,
        })),
        false,
        vec![
            (
                solana_ctx_node_id,
                serde_json::json!({ CTX_EDGE_MARKER: CTX_EDGE_MARKER }),
            ),
            (
                node18,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "account",
                }),
            ),
            (
                node1,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "owner",
                }),
            ),
            (
                node1,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "fee_payer",
                }),
            ),
        ],
    )
    .await;

    let node11 = add_node(
        db.clone(),
        commands::Config::Solana(Kind::Transfer(Transfer {
            fee_payer: None,
            token: None,
            amount: Some(500.0),
            recipient: None,
            sender: None,
            sender_owner: None,
            allow_unfunded_recipient: Some(true),
            fund_recipient: Some(true),
            memo: Some(Some("demo transfer".into())), // todo
        })),
        false,
        vec![
            (
                solana_ctx_node_id,
                serde_json::json!({ CTX_EDGE_MARKER: CTX_EDGE_MARKER }),
            ),
            (
                node9,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "token",
                    INPUT_ARG_NAME_MARKER: "token",
                }),
            ),
            (
                node6,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "sender_owner",
                }),
            ),
            (
                node1,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "sender",
                }),
            ),
            (
                node1,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "fee_payer",
                }),
            ),
            (
                node7,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "recipient",
                }),
            ),
        ],
    )
    .await;

    let node12 = add_node(
        db.clone(),
        commands::Config::Simple(simple::Command::Print),
        false,
        vec![
            (
                solana_ctx_node_id,
                serde_json::json!({ CTX_EDGE_MARKER: CTX_EDGE_MARKER }),
            ),
            (
                node11,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "sender_owner",
                    INPUT_ARG_NAME_MARKER: "print",
                }),
            ),
        ],
    )
    .await;

    let node13 = add_node(
        db.clone(),
        commands::Config::Simple(simple::Command::Print),
        false,
        vec![
            (
                solana_ctx_node_id,
                serde_json::json!({ CTX_EDGE_MARKER: CTX_EDGE_MARKER }),
            ),
            (
                node11,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "recipient",
                    INPUT_ARG_NAME_MARKER: "print",
                }),
            ),
        ],
    )
    .await;

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
//                              6-8  8// print A pubkey
//               5-6   6.create account A          ?  9// mint tokens to A                    6-12  12.// print A balance
// 1-5   5.create minting account                                 6-11&7-11     11// send tokens from A to B
// 14.create minting account keypair          7.create account B                                          7-13  13// print B balance
//                                     7-10     10.// print B pubkey
//
//   5-14    14// print minting account pubkey
