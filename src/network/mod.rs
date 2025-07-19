use crate::Receiver;

use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt::Debug;
use std::hash::Hash;

mod backwards;
mod language;
mod node;
mod signal;

pub use backwards::*;
pub use language::*;
pub use node::*;
pub use signal::*;

/// A type that contains a logic network.
pub trait Network {
    type Gate: Gate;

    /// Returns an iterator containing the ids of the output nodes of the underlying network.
    fn outputs(&self) -> impl Iterator<Item = Signal>;

    /// Returns the node with the given id.
    fn node(&self, id: Id) -> Node<Self::Gate>;

    /// Returns an iterator over all nodes that are reachable from an output and their ids.
    fn iter(&self) -> impl Iterator<Item = (Id, Node<Self::Gate>)> + '_ {
        NetworkNodeIterator {
            network: self,
            visited: FxHashSet::default(),
            remaining: Vec::from_iter(self.outputs().map(|s| s.node_id())),
        }
    }

    /// Sends this network to the given receiver.
    fn send<R: Receiver<Gate = Self::Gate>>(&self, mut receiver: R) -> R::Result {
        let mut src_to_dest: FxHashMap<Id, Signal> = FxHashMap::default();
        let mut path = Vec::new();
        for signal in self.outputs() {
            let mut node_id = signal.node_id();
            let mut node = self.node(node_id);
            let mut known_inputs = 0;
            loop {
                if known_inputs == node.inputs().len() || src_to_dest.contains_key(&node_id) {
                    if known_inputs == node.inputs().len() {
                        let dest_node = node.map_input_ids(|id| src_to_dest[&id]);
                        let dest_signal = receiver.create_node(dest_node);
                        src_to_dest.insert(node_id, dest_signal);
                    }
                    if path.is_empty() {
                        break;
                    }
                    (node_id, node, known_inputs) = path.pop().unwrap();
                    known_inputs += 1;
                } else {
                    let child_id = node.inputs()[known_inputs].node_id();
                    path.push((node_id, node, known_inputs));
                    node_id = child_id;
                    node = self.node(node_id);
                    known_inputs = 0;
                }
            }
        }
        let outputs = Vec::from_iter(
            self.outputs()
                .map(|signal| signal.map_id(|id| src_to_dest[&id])),
        );
        receiver.done(outputs.as_slice())
    }

    fn with_backward_edges(&self) -> impl NetworkWithBackwardEdges<Gate = Self::Gate> + '_ {
        ComputedNetworkWithBackwardEdges::new(self)
    }

    fn dump(&self) {
        for (id, node) in self.iter() {
            println!("{id:?}: {node:?}");
        }
    }
}

struct NetworkNodeIterator<'a, P: ?Sized> {
    network: &'a P,
    visited: FxHashSet<Id>,
    remaining: Vec<Id>,
}

impl<'a, P: Network + ?Sized> Iterator for NetworkNodeIterator<'a, P> {
    type Item = (Id, Node<P::Gate>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let node_id = self.remaining.pop()?;
            if !self.visited.insert(node_id) {
                continue;
            }
            let node = self.network.node(node_id);
            self.remaining
                .extend(node.inputs().iter().map(|s| s.node_id()));
            break Some((node_id, node));
        }
    }
}

/// References a node in a network.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[repr(C)]
pub struct Id(u32);

impl From<egg::Id> for Id {
    fn from(value: egg::Id) -> Self {
        Id(usize::from(value) as u32)
    }
}

impl From<Id> for egg::Id {
    fn from(value: Id) -> Self {
        egg::Id::from(value.0 as usize)
    }
}

impl From<u32> for Id {
    fn from(id: u32) -> Self {
        Id(id)
    }
}

impl From<Id> for u32 {
    fn from(id: Id) -> Self {
        id.0
    }
}
