use crate::Receiver;

use rustc_hash::FxHashMap;
use std::cmp::max;
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
    fanout_nodes: Vec<Id>,
    level: usize,
}

#[derive(Clone)]
pub struct Network<G> {
    // topologically sorted (!) list of nodes where index is node id
    nodes: Vec<NetworkNode<G>>,
    memo: FxHashMap<Node<G>, Id>,
    leaves: Vec<Id>,
    inputs: Vec<Id>,
    outputs: Vec<Signal>,
    max_level: usize,
}

impl<G: Gate> Network<G> {
    /// Returns an array containing the ids of the output nodes of the underlying network.
    pub fn outputs(&self) -> &[Signal] {
        &self.outputs
    }

    pub fn set_outputs(&mut self, outputs: Vec<Signal>) {
        self.outputs = outputs;
    }

    pub fn level(&self, id: Id) -> usize {
        self.nodes[id.to_usize()].level
    }

    pub fn max_level(&self) -> usize {
        self.max_level
    }

    pub fn contains(&self, node: &Node<G>) -> bool {
        self.memo.contains_key(node)
    }

    /// Adds the given node to this network.
    pub fn add(&mut self, node: Node<G>) -> Id {
        if let Some(id) = self.memo.get(&node) {
            return *id;
        }

        if let Node::Input(i) = &node {
            for j in self.inputs.len() as u32..*i {
                self.add(Node::Input(j));
            }
        }

        let id = Id::from(self.nodes.len() as u32);
        if node.is_leaf() {
            self.leaves.push(id);
            if let Node::Input(i) = &node
                && *i >= self.inputs.len() as u32
            {
                assert_eq!(*i, self.inputs.len() as u32);
                self.inputs.push(id);
            };
        } else {
            if node
                .inputs()
                .iter()
                .any(|input| input.node_id().to_usize() >= id.to_usize())
            {
                panic!("node input id out of bounds");
            }
            for (i, input) in node.inputs().iter().enumerate() {
                if !node.inputs()[0..i].iter().any(|prev| prev == input) {
                    self.nodes[input.node_id().to_usize()]
                        .fanout
                        .push(Signal::new(id, input.is_inverted()));
                }
                if !node.inputs()[0..i]
                    .iter()
                    .any(|prev| prev.node_id() == input.node_id())
                {
                    self.nodes[input.node_id().to_usize()].fanout_nodes.push(id);
                }
            }
        }

        let level = node
            .inputs()
            .iter()
            .map(|signal| self.nodes[signal.node_id().to_usize()].level + 1)
            .max()
            .unwrap_or(0);
        self.max_level = max(self.max_level, level);
        self.memo.insert(node.clone(), id);
        self.nodes.push(NetworkNode {
            node,
            fanout: Vec::new(),
            fanout_nodes: Vec::new(),
            level,
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

    /// Returns the *set* of node ids that have the node with the given id as an input.
    pub fn node_output_ids(&self, id: Id) -> &[Id] {
        &self.nodes[id.to_usize()].fanout_nodes
    }

    /// Returns an iterator over all leaf nodes (i.e. nodes with no inputs).
    pub fn leaves(&self) -> &[Id] {
        &self.leaves
    }

    pub fn inputs(&self) -> &[Id] {
        &self.inputs
    }

    /// Sends this network to the given receiver.
    pub fn send<R: Receiver<Gate = G>>(mut self, mut receiver: R) -> R::Result {
        let mut src_to_dest: FxHashMap<Id, Signal> = FxHashMap::default();
        let mut outputs = std::mem::take(&mut self.outputs);
        for (id, node) in self {
            let mapped_node = node.map_input_ids(|id, _| src_to_dest[&id]);
            let signal = receiver.create(mapped_node);
            src_to_dest.insert(id, signal);
        }
        for output in &mut outputs {
            *output = output.map_id(|id| src_to_dest[&id]);
        }
        receiver.done(outputs)
    }

    pub fn dump(&self) {
        for (id, node) in self.iter() {
            println!("{id:?}: {node:?}");
        }
        print!("outputs:");
        for output in self.outputs() {
            print!(" {output:?}")
        }
        println!()
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
            inputs: Default::default(),
            max_level: 0,
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
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
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
