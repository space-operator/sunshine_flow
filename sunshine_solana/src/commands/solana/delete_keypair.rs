use std::{collections::HashMap, sync::Arc};

use either::Either;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use sunshine_core::msg::NodeId;

use crate::{error::Error, ValueType};

use super::Ctx;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeleteKeypair {
    pub name: Either<Option<String>, Option<NodeId>>,
}

impl DeleteKeypair {
    pub(crate) async fn run(
        &self,
        ctx: Arc<Ctx>,
        mut inputs: HashMap<String, ValueType>,
    ) -> Result<HashMap<String, ValueType>, Error> {
        match &self.name {
            Either::Left(name) => {
                let name = match name {
                    Some(s) => s.clone(),
                    None => match inputs.remove("name") {
                        Some(ValueType::String(s)) => s,
                        _ => return Err(Error::ArgumentNotFound("name".to_string())),
                    },
                };
                Self::delete_from_name(ctx, name).await
            }
            Either::Right(node_id) => {
                let node_id = match node_id {
                    Some(id) => *id,
                    None => match inputs.remove("node_id") {
                        Some(ValueType::NodeId(id)) => id,
                        _ => return Err(Error::ArgumentNotFound("node_id".to_string())),
                    },
                };

                Self::delete_from_node_id(ctx, node_id).await
            }
        }
    }

    async fn delete_from_name(
        ctx: Arc<Ctx>,
        name: String,
    ) -> Result<HashMap<String, ValueType>, Error> {
        let graph = ctx.db.read_graph(ctx.key_graph).await?;

        for node in graph.nodes {
            for edge in node.inbound_edges {
                let props = ctx.db.read_edge_properties(edge).await?;
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
        ctx: Arc<Ctx>,
        node_id: NodeId,
    ) -> Result<HashMap<String, ValueType>, Error> {
        ctx.db.delete_node(node_id, ctx.key_graph).await?;

        ctx.keyring.find_remove(|(name, keypair)| if keypair.)

        Ok(hashmap! {
            "deleted_node_id".to_owned()=> ValueType::DeletedNode(node_id)
        })
    }
}
