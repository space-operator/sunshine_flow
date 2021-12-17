use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use either::Either;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use sunshine_core::msg::NodeId;

use crate::{error::Error, Msg};

use super::Ctx;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeleteKeypair {
    pub name: Either<Option<String>, Option<NodeId>>,
}

impl DeleteKeypair {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Mutex<Ctx>>,
        mut inputs: HashMap<String, Msg>,
    ) -> Result<HashMap<String, Msg>, Error> {
        match &self.name {
            Either::Left(name) => {
                let name = match name {
                    Some(s) => s.clone(),
                    None => match inputs.remove("name") {
                        Some(Msg::String(s)) => s,
                        _ => return Err(Error::ArgumentNotFound("name".to_string())),
                    },
                };
                Self::delete_from_name(ctx, name).await
            }
            Either::Right(node_id) => {
                let node_id = match node_id {
                    Some(id) => *id,
                    None => match inputs.remove("node_id") {
                        Some(Msg::NodeId(id)) => id,
                        _ => return Err(Error::ArgumentNotFound("node_id".to_string())),
                    },
                };
                Self::delete_from_node_id(ctx, node_id).await
            }
        }
    }

    async fn delete_from_name(
        ctx: Arc<Mutex<Ctx>>,
        name: String,
    ) -> Result<HashMap<String, Msg>, Error> {
        let graph = ctx
            .lock()
            .unwrap()
            .db
            .read_graph(ctx.lock().unwrap().key_graph)
            .await?;

        for node in graph.nodes {
            for edge in node.inbound_edges {
                let props = ctx.lock().unwrap().db.read_edge_properties(edge).await?;
                match props.get(super::KEYPAIR_NAME_MARKER) {
                    Some(n) if n == name.as_str() => {
                        return Self::delete_from_node_id(ctx, node.node_id).await;
                    }
                    _ => (),
                }
            }
        }

        return Err(Error::KeypairDoesntExist);
    }

    async fn delete_from_node_id(
        ctx: Arc<Mutex<Ctx>>,
        node_id: NodeId,
    ) -> Result<HashMap<String, Msg>, Error> {
        ctx.lock()
            .unwrap()
            .db
            .delete_node(node_id, ctx.lock().unwrap().key_graph)
            .await?;

        Ok(hashmap! {
            "deleted_node".to_owned()=>Msg::DeletedNode(node_id)
        })
    }
}
