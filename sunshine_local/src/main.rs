use std::sync::Arc;
use std::time::Duration;

use serde_json::Value as JsonValue;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use sunshine_core::msg::{CreateEdge, MutateKind, NodeId};
use sunshine_core::{msg::Action, store::Datastore};
use sunshine_indra::store::{DbConfig, DB};
use sunshine_solana::commands::simple;
use sunshine_solana::commands::simple::http_request::HttpRequest;
use sunshine_solana::commands::simple::ipfs_upload::IpfsUpload;
use sunshine_solana::commands::simple::json_extract::JsonExtract;
use sunshine_solana::commands::solana::get_balance::GetBalance;
use sunshine_solana::commands::solana::nft::approve_use_authority::ApproveUseAuthority;
use sunshine_solana::commands::solana::nft::arweave_upload::ArweaveUpload;
use sunshine_solana::commands::solana::nft::create_master_edition::{
    Arg as MasterEditionArg, CreateMasterEdition,
};
use sunshine_solana::commands::solana::nft::create_metadata_accounts::{
    CreateMetadataAccounts, NftCollection, NftUseMethod, NftUses,
};
use sunshine_solana::commands::solana::nft::get_left_uses::GetLeftUses;
use sunshine_solana::commands::solana::nft::update_metadata_accounts::{
    MetadataAccountData, UpdateMetadataAccounts,
};
use sunshine_solana::commands::solana::nft::utilize::Utilize;
use sunshine_solana::commands::solana::request_airdrop::RequestAirdrop;
use sunshine_solana::commands::solana::transfer::Transfer;
use sunshine_solana::commands::solana::{self, nft, Kind};
use sunshine_solana::{
    commands, FlowContext, NftCreator, Schedule, COMMAND_MARKER, COMMAND_NAME_MARKER,
    CTX_EDGE_MARKER, CTX_MARKER, INPUT_ARG_NAME_MARKER, OUTPUT_ARG_NAME_MARKER, START_NODE_MARKER,
};

use sunshine_solana::commands::solana::create_account::CreateAccount;
use sunshine_solana::commands::solana::create_token::CreateToken;
use sunshine_solana::commands::solana::generate_keypair::GenerateKeypair;
use sunshine_solana::commands::solana::mint_token::MintToken;

/*
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

    let simple_command =
        simple::Command::Const(sunshine_solana::Value::StringOpt(Some(seed.into())));

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
        seed_phrase: solana::generate_keypair::Arg::None,
        passphrase: Some("pass".into()),
        save: solana::generate_keypair::Arg::Some(None),
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

    db.execute(Action::Mutate(
        flow_graph_id,
        MutateKind::CreateEdge(CreateEdge {
            from: node1,
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

    let add_solana_node = |db: Arc<DB>,
                           cfg: commands::Config,
                           is_start_node: bool,
                           mut inbound_edges: Vec<(NodeId, JsonValue)>| async move {
        inbound_edges.push((
            solana_ctx_node_id,
            serde_json::json!({ CTX_EDGE_MARKER: CTX_EDGE_MARKER }),
        ));
        add_node(db, cfg, is_start_node, inbound_edges).await
    };

    // used //kiss february ivory merge topic uncover female cancel innocent leg surprise cabbage
    // laugh toy good ring measure position random squirrel penalty prosper write liar
    // must motor sail initial budget moral drip asthma slide steak since lesson
    // used // hello deer force person lunch wonder cash crater happy security punch decade

    let node14 = add_solana_node(
        db.clone(),
        commands::Config::Solana(Kind::GenerateKeypair(GenerateKeypair {
            seed_phrase: solana::generate_keypair::Arg::Some(None),
            passphrase: Some("asdasdas".into()),
            save: solana::generate_keypair::Arg::Some(None),
        })),
        true,
        vec![],
    )
    .await;

    let node5 = add_solana_node(
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

    let node17 = add_solana_node(
        db.clone(),
        commands::Config::Solana(Kind::GenerateKeypair(GenerateKeypair {
            seed_phrase: solana::generate_keypair::Arg::Some(None),
            passphrase: Some("123123".into()),
            save: solana::generate_keypair::Arg::Some(None),
        })),
        true,
        vec![],
    )
    .await;

    let node6 = add_solana_node(
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

    let node8 = add_solana_node(
        db.clone(),
        commands::Config::Simple(simple::Command::Print),
        false,
        vec![(
            node17,
            serde_json::json!({
                OUTPUT_ARG_NAME_MARKER: "pubkey",
                INPUT_ARG_NAME_MARKER: "print",
            }),
        )],
    )
    .await;

    let node15 = add_node(
        db.clone(),
        commands::Config::Simple(simple::Command::Print),
        false,
        vec![(
            node5,
            serde_json::json!({
                OUTPUT_ARG_NAME_MARKER: "token",
                INPUT_ARG_NAME_MARKER: "print",
            }),
        )],
    )
    .await;

    let node9 = add_solana_node(
        db.clone(),
        commands::Config::Solana(Kind::MintToken(MintToken {
            token: None,
            recipient: None,
            mint_authority: None,
            amount: Some(501.23456),
            fee_payer: None,
        })),
        false,
        vec![
            (
                node6,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "account",
                    INPUT_ARG_NAME_MARKER: "recipient",
                }),
            ),
            (
                node6,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "fee_payer",
                    INPUT_ARG_NAME_MARKER: "mint_authority",
                }),
            ),
            (
                node6,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "fee_payer",
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

    let node18 = add_solana_node(
        db.clone(),
        commands::Config::Solana(Kind::GenerateKeypair(GenerateKeypair {
            seed_phrase: solana::generate_keypair::Arg::Some(None),
            passphrase: Some("pass".into()),
            save: solana::generate_keypair::Arg::Some(None),
        })),
        true,
        vec![],
    )
    .await;

    let node10 = add_node(
        db.clone(),
        commands::Config::Simple(simple::Command::Print),
        false,
        vec![(
            node17,
            serde_json::json!({
                OUTPUT_ARG_NAME_MARKER: "pubkey",
                INPUT_ARG_NAME_MARKER: "print",
            }),
        )],
    )
    .await;

    let node11 = add_solana_node(
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
                node18,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "pubkey",
                    INPUT_ARG_NAME_MARKER: "recipient",
                }),
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
                    OUTPUT_ARG_NAME_MARKER: "fee_payer",
                    INPUT_ARG_NAME_MARKER: "sender_owner",
                }),
            ),
            (
                node6,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "account",
                    INPUT_ARG_NAME_MARKER: "sender",
                }),
            ),
            (
                node6,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "fee_payer",
                    INPUT_ARG_NAME_MARKER: "fee_payer",
                }),
            ),
        ],
    )
    .await;

    let node21 = add_node(
        db.clone(),
        commands::Config::Simple(simple::Command::Print),
        false,
        vec![(
            node11,
            serde_json::json!({
                OUTPUT_ARG_NAME_MARKER: "recipient_acc",
                INPUT_ARG_NAME_MARKER: "print",
            }),
        )],
    )
    .await;

    // deploy
    flow_context
        .deploy_flow(Duration::from_secs(100000000000), flow_graph_id)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(100)).await;

    // create flow graph
    // create solana context nodes
    // add commands
    // connect commands

    // create flow context

    // deploy flow
}
*/
//                              6-8  8// print A pubkey
//               5-6   6.create account A          ?  9// mint tokens to A                    6-12  12.// print A balance
// 1-5   5.create minting account                                 6-11&7-11     11// send tokens from A to B
// 14.create minting account keypair          7.create account B                                          7-13  13// print B balance
//                                     7-10     10.// print B pubkey
//
//   5-14    14// print minting account pubkey

#[tokio::main]
async fn main() {
    let db_config = DbConfig {
        db_path: "flow_db2".into(),
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
        solana_url: "https://api.devnet.solana.com".into(),
        solana_arweave_url: "https://arloader.io/dev".into(),
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

    // create command nodes
    let add_node = |db: Arc<DB>,
                    name: String,
                    cfg: commands::Config,
                    is_start_node: bool,
                    inbound_edges: Vec<(NodeId, JsonValue)>| async move {
        let mut props = serde_json::Map::new();

        props.insert(COMMAND_MARKER.into(), serde_json::to_value(cfg).unwrap());
        props.insert(
            COMMAND_NAME_MARKER.into(),
            JsonValue::String(name.to_owned()),
        );

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

    let add_solana_node = |db: Arc<DB>,
                           name: String,
                           cfg: commands::Config,
                           is_start_node: bool,
                           mut inbound_edges: Vec<(NodeId, JsonValue)>| async move {
        inbound_edges.push((
            solana_ctx_node_id,
            serde_json::json!({ CTX_EDGE_MARKER: CTX_EDGE_MARKER }),
        ));
        add_node(db, name, cfg, is_start_node, inbound_edges).await
    };

    let node0 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::GenerateKeypair(GenerateKeypair {
            seed_phrase: solana::generate_keypair::Arg::Some(None),
            passphrase: Some("123123".into()),
            save: solana::generate_keypair::Arg::Some(None),
            base58_str: solana::generate_keypair::Arg::Some(None),
        })),
        true,
        vec![],
    )
    .await;

    let node1 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::RequestAirdrop(RequestAirdrop {
            pubkey: None,
            amount: Some(50_000_000),
        })),
        false,
        vec![(
            node0,
            serde_json::json!({
                OUTPUT_ARG_NAME_MARKER: "pubkey",
                INPUT_ARG_NAME_MARKER: "pubkey",
            }),
        )],
    )
    .await;

    /*

    let node2 = add_node(
        db.clone(),
        "".into(),
        commands::Config::Simple(simple::Command::Print),
        false,
        vec![(
            node0,
            serde_json::json!({
                OUTPUT_ARG_NAME_MARKER: "pubkey",
                INPUT_ARG_NAME_MARKER: "print",
            }),
        )],
    )
    .await;

    let node3 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::GenerateKeypair(GenerateKeypair {
            seed_phrase: solana::generate_keypair::Arg::Some(None),
            passphrase: Some("asdasdas".into()),
            save: solana::generate_keypair::Arg::Some(None),
        })),
        false,
        vec![(
            node1,
            serde_json::json!({
                OUTPUT_ARG_NAME_MARKER: "signature",
                INPUT_ARG_NAME_MARKER: "signature",
            }),
        )],
    )
    .await;

    let node4 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::CreateToken(CreateToken {
            fee_payer: None,
            decimals: Some(0),
            authority: None,
            token: None,
            memo: Some("SUNSHINE NFT MINTING ACCOUNT".into()),
        })),
        false,
        vec![
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "fee_payer",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "authority",
                }),
            ),
            (
                node3,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "token",
                }),
            ),
        ],
    )
    .await;

    let node8 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::GenerateKeypair(GenerateKeypair {
            seed_phrase: solana::generate_keypair::Arg::Some(None),
            passphrase: Some("qweqwew".into()),
            save: solana::generate_keypair::Arg::Some(None),
        })),
        true,
        vec![],
    )
    .await;

    let node9 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::CreateAccount(CreateAccount {
            owner: None,
            fee_payer: None,
            token: None,
            account: None,
        })),
        false,
        vec![
            (
                node8,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "account",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "owner",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "fee_payer",
                }),
            ),
            (
                node4,
                serde_json::json!({
                    INPUT_ARG_NAME_MARKER: "token",
                    OUTPUT_ARG_NAME_MARKER: "token",
                }),
            ),
        ],
    )
    .await;

    let node10 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::MintToken(MintToken {
            token: None,
            recipient: None,
            mint_authority: None,
            amount: Some(1.),
            fee_payer: None,
        })),
        false,
        vec![
            (
                node9,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "account",
                    INPUT_ARG_NAME_MARKER: "recipient",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "mint_authority",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "fee_payer",
                }),
            ),
            (
                node4,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "token",
                    INPUT_ARG_NAME_MARKER: "token",
                }),
            ),
        ],
    )
    .await;

    let node23 = add_node(
        db.clone(),
        "".into(),
        commands::Config::Simple(simple::Command::HttpRequest(HttpRequest {
            method: Some("GET".into()),
            url: Some(
                "https://api.airtable.com/v0/appRYVa2YoZdNsVkk/Table%201/recWNxPJAJ4qmz1On".into(),
            ),
            auth_token: Some("keynH6Eh9ZN1Y2oqw".into()),
        })),
        true,
        vec![],
    )
    .await;

    let node24 = add_node(
        db.clone(),
        "".into(),
        commands::Config::Simple(simple::Command::JsonExtract(JsonExtract {
            pointer: "/fields/Url".into(),
            arg: "body".into(),
        })),
        false,
        vec![(
            node23,
            serde_json::json!({
                OUTPUT_ARG_NAME_MARKER: "resp_body",
                INPUT_ARG_NAME_MARKER: "body",
            }),
        )],
    )
    .await;

    let node6 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::Nft(nft::Command::CreateMetadataAccounts(
            CreateMetadataAccounts {
                token: None,
                token_authority: None,
                fee_payer: None,        // keypair
                update_authority: None, // keypair
                name: Some("SUNSHINE_TICKET_NFT".into()),
                symbol: Some("SUNFTT".into()),
                uri: None,
                creators: None,
                seller_fee_basis_points: Some(420),
                update_authority_is_signer: Some(true),
                is_mutable: Some(true),
                collection: None,
                uses: Some(Some(NftUses {
                    remaining: 6,
                    total: 6,
                    use_method: NftUseMethod::Multiple,
                })),
            },
        ))),
        false,
        vec![
            (
                node24,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "val",
                    INPUT_ARG_NAME_MARKER: "uri",
                }),
            ),
            (
                node4,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "token",
                    INPUT_ARG_NAME_MARKER: "token",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "pubkey",
                    INPUT_ARG_NAME_MARKER: "token_authority",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "fee_payer",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "update_authority",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "pubkey",
                    INPUT_ARG_NAME_MARKER: "creators",
                }),
            ),
            (
                node10,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "signature",
                    INPUT_ARG_NAME_MARKER: "signature",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "empty",
                    INPUT_ARG_NAME_MARKER: "collection",
                }),
            ),
        ],
    )
    .await;

    let node11 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::Nft(nft::Command::CreateMasterEdition(
            CreateMasterEdition {
                token: None,
                token_authority: None,
                fee_payer: None,        // keypair
                update_authority: None, // keypair
                is_mutable: Some(false),
                max_supply: MasterEditionArg::Some(Some(5)),
            },
        ))),
        false,
        vec![
            (
                node4,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "token",
                    INPUT_ARG_NAME_MARKER: "token",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "pubkey",
                    INPUT_ARG_NAME_MARKER: "token_authority",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "fee_payer",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "update_authority",
                }),
            ),
            (
                node6,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "signature",
                    INPUT_ARG_NAME_MARKER: "signature",
                }),
            ),
        ],
    )
    .await;

    let node7 = add_node(
        db.clone(),
        "".into(),
        commands::Config::Simple(simple::Command::Print),
        false,
        vec![(
            node6,
            serde_json::json!({
                OUTPUT_ARG_NAME_MARKER: "metadata_pubkey",
                INPUT_ARG_NAME_MARKER: "print",
            }),
        )],
    )
    .await;

    let node12 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::Nft(nft::Command::UpdateMetadataAccounts(
            UpdateMetadataAccounts {
                token: None,
                fee_payer: None,        // keypair
                update_authority: None, // keypair
                new_update_authority: None,
                data: Some(Some(MetadataAccountData {
                    name: "SUNSHINE_TICKET_NFT2".into(),
                    symbol: "SUNFTT".into(),
                    uri: "https://api.jsonbin.io/b/61ddc9072675917a628edc21".into(),
                    creators: None,
                    seller_fee_basis_points: 425,
                    collection: None,
                    uses: Some(NftUses {
                        remaining: 3,
                        total: 3,
                        use_method: NftUseMethod::Multiple,
                    }),
                })),
                primary_sale_happened: Some(Some(true)),
                is_mutable: Some(Some(false)),
            },
        ))),
        false,
        vec![
            (
                node4,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "token",
                    INPUT_ARG_NAME_MARKER: "token",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "fee_payer",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "update_authority",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "pubkey",
                    INPUT_ARG_NAME_MARKER: "new_update_authority",
                }),
            ),
            (
                node11,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "signature",
                    INPUT_ARG_NAME_MARKER: "signature",
                }),
            ),
        ],
    )
    .await;

    let node17 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::GenerateKeypair(GenerateKeypair {
            seed_phrase: solana::generate_keypair::Arg::Some(None),
            passphrase: Some("qweqwew".into()),
            save: solana::generate_keypair::Arg::Some(None),
        })),
        true,
        vec![],
    )
    .await;

    let node18 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::Nft(nft::Command::ApproveUseAuthority(
            ApproveUseAuthority {
                user: None,
                owner: None,
                fee_payer: None,
                token_account: None,
                token: None,
                burner: None,
                number_of_uses: Some(2),
            },
        ))),
        false,
        vec![
            (
                node17,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "pubkey",
                    INPUT_ARG_NAME_MARKER: "user",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "owner",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "fee_payer",
                }),
            ),
            (
                node9,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "account",
                    INPUT_ARG_NAME_MARKER: "token_account",
                }),
            ),
            (
                node4,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "token",
                    INPUT_ARG_NAME_MARKER: "token",
                }),
            ),
            (
                node9,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "account",
                    INPUT_ARG_NAME_MARKER: "burner",
                }),
            ),
            (
                node12,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "signature",
                    INPUT_ARG_NAME_MARKER: "signature",
                }),
            ),
        ],
    )
    .await;

    let node19 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::Nft(nft::Command::GetLeftUses(GetLeftUses {
            token: None,
        }))),
        false,
        vec![
            (
                node4,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "token",
                    INPUT_ARG_NAME_MARKER: "token",
                }),
            ),
            (
                node18,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "signature",
                    INPUT_ARG_NAME_MARKER: "signature",
                }),
            ),
        ],
    )
    .await;

    let node20 = add_node(
        db.clone(),
        "".into(),
        commands::Config::Simple(simple::Command::Print),
        false,
        vec![(
            node19,
            serde_json::json!({
                OUTPUT_ARG_NAME_MARKER: "left_uses",
                INPUT_ARG_NAME_MARKER: "print",
            }),
        )],
    )
    .await;

    let node16 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::Nft(nft::Command::Utilize(Utilize {
            token_account: None,
            token: None,
            use_authority_record_pda: None,
            use_authority: None,
            fee_payer: None,
            owner: None,
            burner: None,
            number_of_uses: Some(2),
        }))),
        false,
        vec![
            (
                node9,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "account",
                    INPUT_ARG_NAME_MARKER: "token_account",
                }),
            ),
            (
                node4,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "token",
                    INPUT_ARG_NAME_MARKER: "token",
                }),
            ),
            (
                node18,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "use_authority_record_pubkey",
                    INPUT_ARG_NAME_MARKER: "use_authority_record_pda",
                }),
            ),
            (
                node17,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "use_authority",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "fee_payer",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "owner",
                }),
            ),
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "empty",
                    INPUT_ARG_NAME_MARKER: "burner",
                }),
            ),
            (
                node19,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "left_uses",
                    INPUT_ARG_NAME_MARKER: "left_uses",
                }),
            ),
        ],
    )
    .await;

    let node21 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::Nft(nft::Command::GetLeftUses(GetLeftUses {
            token: None,
        }))),
        false,
        vec![
            (
                node4,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "token",
                    INPUT_ARG_NAME_MARKER: "token",
                }),
            ),
            (
                node16,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "signature",
                    INPUT_ARG_NAME_MARKER: "signature",
                }),
            ),
        ],
    )
    .await;

    let node22 = add_node(
        db.clone(),
        "".into(),
        commands::Config::Simple(simple::Command::Print),
        false,
        vec![(
            node21,
            serde_json::json!({
                OUTPUT_ARG_NAME_MARKER: "left_uses",
                INPUT_ARG_NAME_MARKER: "print",
            }),
        )],
    )
    .await;

    let node23 = add_node(
        db.clone(),
        "".into(),
        commands::Config::Simple(simple::Command::IpfsUpload(IpfsUpload {
            pinata_url: Some("https://api.pinata.cloud".into()),
            // pinata_key: Some("bacddb411bd45dd3a531".into()),
            // pinata_secret: Some(
            //     "19daa373cd4537b4c78742c9dd4b75550b89cfd69a49b8c491a744933c85680b".into(),
            // ),
            file_path: Some("image.jpg".into()),
            pinata_jwt: Some("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VySW5mb3JtYXRpb24iOnsiaWQiOiI1NmVjMTNhOS03NTYyLTRiZjMtODgzOS00ZjVlY2YxOTFmMDYiLCJlbWFpbCI6ImVuem90YXIzMDAwQGdtYWlsLmNvbSIsImVtYWlsX3ZlcmlmaWVkIjp0cnVlLCJwaW5fcG9saWN5Ijp7InJlZ2lvbnMiOlt7ImlkIjoiTllDMSIsImRlc2lyZWRSZXBsaWNhdGlvbkNvdW50IjoxfV0sInZlcnNpb24iOjF9LCJtZmFfZW5hYmxlZCI6ZmFsc2V9LCJhdXRoZW50aWNhdGlvblR5cGUiOiJzY29wZWRLZXkiLCJzY29wZWRLZXlLZXkiOiJiYWNkZGI0MTFiZDQ1ZGQzYTUzMSIsInNjb3BlZEtleVNlY3JldCI6IjE5ZGFhMzczY2Q0NTM3YjRjNzg3NDJjOWRkNGI3NTU1MGI4OWNmZDY5YTQ5YjhjNDkxYTc0NDkzM2M4NTY4MGIiLCJpYXQiOjE2NDQ5NTQ0ODl9.9AUY-lYSMpWSS7IQcnkv52J_MYiPDhagWbUT2rv7yTk".into()),
        })),
        true,
        vec![],
    )
    .await;

    let node24 = add_node(
        db.clone(),
        "".into(),
        commands::Config::Simple(simple::Command::Print),
        false,
        vec![(
            node23,
            serde_json::json!({
                OUTPUT_ARG_NAME_MARKER: "image_cid",
                INPUT_ARG_NAME_MARKER: "print",
            }),
        )],
    )
    .await;
    */

    let node25 = add_solana_node(
        db.clone(),
        "".into(),
        commands::Config::Solana(Kind::Nft(nft::Command::ArweaveUpload(ArweaveUpload {
            fee_payer: None,
            reward_mult: Some(10.),
            file_path: Some("image.jpg".into()),
        }))),
        false,
        vec![
            (
                node0,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "keypair",
                    INPUT_ARG_NAME_MARKER: "fee_payer",
                }),
            ),
            (
                node1,
                serde_json::json!({
                    OUTPUT_ARG_NAME_MARKER: "signature",
                    INPUT_ARG_NAME_MARKER: "signature",
                }),
            ),
        ],
    )
    .await;

    let node26 = add_node(
        db.clone(),
        "".into(),
        commands::Config::Simple(simple::Command::Print),
        false,
        vec![(
            node25,
            serde_json::json!({
                OUTPUT_ARG_NAME_MARKER: "file_uri",
                INPUT_ARG_NAME_MARKER: "print",
            }),
        )],
    )
    .await;

    flow_context
        .deploy_flow(Schedule::Once, flow_graph_id)
        .await
        .unwrap();

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(15)).await;

            let flow_node = db.read_node(flow_graph_id).await.unwrap();

            for edge in flow_node.outbound_edges {
                let props = db.read_edge_properties(edge).await.unwrap();

                if props.contains_key("timestamp") {
                    let log_graph = db.read_graph(edge.to).await.unwrap();
                    println!("{:#?}", log_graph);
                }
            }
        }
    });

    tokio::time::sleep(Duration::from_secs(1000)).await;
}

/*
#[tokio::main]
async fn main() {
    let db_config = DbConfig {
        db_path: "/home/amir/SUNSHINE_DB".into(),
    };
    let db = DB::new(&db_config).unwrap();
    let db = Arc::new(db);

    let flow_context = FlowContext::new(db.clone());

    let flow_graph_id = uuid::Uuid::from_str("ebea9a5d-89d4-11ec-8000-000000000000").unwrap();

    flow_context
        .deploy_flow(Schedule::Once, flow_graph_id)
        .await
        .unwrap();

    let mut interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        interval.tick().await;

        let flow_node = db.read_node(flow_graph_id).await.unwrap();

        for edge in flow_node.outbound_edges {
            let props = db.read_edge_properties(edge).await.unwrap();

            if props.contains_key("timestamp") {
                let log_graph = db.read_graph(edge.to).await.unwrap();
                println!("{:#?}", log_graph);
            }
        }
    }
}
*/
