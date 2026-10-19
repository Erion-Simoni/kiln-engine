//! Graph data model.

use crate::{Connection, Node};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A node graph (collection of nodes and connections).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: HashMap<Uuid, Node>,
    pub connections: Vec<Connection>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            connections: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: Node) -> Uuid {
        let id = node.id;
        self.nodes.insert(id, node);
        id
    }

    pub fn remove_node(&mut self, id: Uuid) -> Option<Node> {
        // Remove connections to/from this node
        if let Some(node) = self.nodes.get(&id) {
            let pin_ids: Vec<Uuid> = node.pins.iter().map(|p| p.id).collect();
            self.connections
                .retain(|c| !pin_ids.contains(&c.source_pin) && !pin_ids.contains(&c.target_pin));
        }
        self.nodes.remove(&id)
    }

    pub fn add_connection(&mut self, connection: Connection) {
        self.connections.push(connection);
    }

    pub fn remove_connection(&mut self, id: Uuid) -> bool {
        if let Some(pos) = self.connections.iter().position(|c| c.id == id) {
            self.connections.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn get_node(&self, id: Uuid) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn get_pin_node(&self, pin_id: Uuid) -> Option<&Node> {
        self.nodes
            .values()
            .find(|n| n.pins.iter().any(|p| p.id == pin_id))
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}
