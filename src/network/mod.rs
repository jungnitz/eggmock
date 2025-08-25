use crate::Receiver;

use rustc_hash::FxHashMap;
use std::fmt::Debug;
use std::hash::Hash;

mod language;
mod node;
mod signal;

pub use language::*;
pub use node::*;
pub use signal::*;

#[derive(Clone)]
struct NetworkNode<G> {
    node: Node<G>,
    fanout: Vec<Signal>,
}

#[derive(Clone)]
pub struct Network<G> {
    // topologically sorted (!) list of nodes where index is node id
    nodes: Vec<NetworkNode<G>>,
    memo: FxHashMap<Node<G>, Id>,
    leaves: Vec<Id>,
    outputs: Vec<Signal>,
}

impl<G: Gate> Network<G> {
    /// Returns an array containing the ids of the output nodes of the underlying network.
    pub fn outputs(&self) -> &[Signal] {
        &self.outputs
    }

    pub fn set_outputs(&mut self, outputs: Vec<Signal>) {
        self.outputs = outputs;
    }

    /// Adds the given node to this network.
    pub fn add(&mut self, node: Node<G>) -> Id {
        if let Some(id) = self.memo.get(&node) {
            return *id;
        }

        let id = Id::from(self.nodes.len() as u32);
        if node.is_leaf() {
            self.leaves.push(id);
        } else {
            if node
                .inputs()
                .iter()
                .any(|input| input.node_id().to_usize() >= id.to_usize())
            {
                panic!("node input id out of bounds");
            }
            for (i, input) in node.inputs().iter().enumerate() {
                if node.inputs()[0..i].iter().any(|prev| prev == input) {
                    continue;
                }
                self.nodes[input.node_id().to_usize()]
                    .fanout
                    .push(Signal::new(id, input.is_inverted()));
            }
        }
        self.memo.insert(node.clone(), id);
        self.nodes.push(NetworkNode {
            node,
            fanout: Vec::new(),
        });
        id
    }

    /// Returns the node with the given id.
    pub fn node(&self, id: Id) -> &Node<G> {
        &self.nodes[id.to_usize()].node
    }

    /// Returns an iterator over all nodes.
    pub fn iter(&self) -> impl Iterator<Item = (Id, &Node<G>)> + '_ {
        self.nodes
            .iter()
            .enumerate()
            .map(|(id, node)| (Id::from_usize(id), &node.node))
    }

    /// Returns the *set* of signals pointing *to* nodes that have the node with the given id as an
    /// input.
    pub fn node_outputs(&self, id: Id) -> &[Signal] {
        &self.nodes[id.to_usize()].fanout
    }

    /// Returns an iterator over all leaf nodes (i.e. nodes with no inputs).
    pub fn leaves(&self) -> &[Id] {
        &self.leaves
    }

    /// Sends this network to the given receiver.
    pub fn send<R: Receiver<Gate = G>>(mut self, mut receiver: R) -> R::Result {
        let mut src_to_dest: FxHashMap<Id, Signal> = FxHashMap::default();
        let outputs = std::mem::take(&mut self.outputs);
        for (id, node) in self {
            let mapped_node = node.map_input_ids(|id, _| src_to_dest[&id]);
            let signal = receiver.create(mapped_node);
            src_to_dest.insert(id, signal);
        }
        receiver.done(outputs)
    }

    pub fn dump(&self) {
        for (id, node) in self.iter() {
            println!("{id:?}: {node:?}");
        }
    }

    pub fn size(&self) -> usize {
        self.nodes.len()
    }
}

impl<G> Default for Network<G> {
    fn default() -> Self {
        Self {
            leaves: Default::default(),
            memo: Default::default(),
            nodes: Default::default(),
            outputs: Default::default(),
        }
    }
}

pub struct IntoIter<G>(usize, std::vec::IntoIter<NetworkNode<G>>);

impl<G> IntoIterator for Network<G> {
    type Item = (Id, Node<G>);
    type IntoIter = IntoIter<G>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(0, self.nodes.into_iter())
    }
}

impl<G> Iterator for IntoIter<G> {
    type Item = (Id, Node<G>);

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.1.next()?;
        let id = Id::from_usize(self.0);
        self.0 += 1;
        Some((id, next.node))
    }
}

/// References a node in a network.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
#[repr(C)]
pub struct Id(u32);

impl Id {
    pub fn to_usize(self) -> usize {
        self.0 as usize
    }
    pub fn from_usize(id: usize) -> Id {
        Self(id as u32)
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

pub struct NetworkReceiver<G>(Network<G>);

impl<G: Gate> Receiver for NetworkReceiver<G> {
    type Gate = G;
    type Result = Network<G>;

    fn create(&mut self, node: Node<Self::Gate>) -> Signal {
        Signal::new(self.0.add(node), false)
    }

    fn done(mut self, outputs: Vec<Signal>) -> Self::Result {
        self.0.set_outputs(outputs);
        self.0
    }
}

impl<G> Default for NetworkReceiver<G> {
    fn default() -> Self {
        Self(Default::default())
    }
}
