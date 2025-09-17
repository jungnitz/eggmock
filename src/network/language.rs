use egg::Language;

use crate::{Gate, GateFunction, Node, Signal};

/// Contains the [`Language`] type that can represent a Network.
pub trait NetworkLanguage: Language {
    type Gate: Gate;

    /// Creates an instance of this type given a node of the network. The input signals are mapped
    /// to child ids with the given mapper.
    fn from_node(
        node: Node<Self::Gate>,
        signal_mapper: impl FnMut(Signal, usize) -> egg::Id,
    ) -> Self;
    /// Creates a network node from this EGraph node. The child ids are mapped to signals with the
    /// given mapper. This mapper will usually resolve the nots before the next real network node.
    ///
    /// Returns [`None`] if this EGraph node is a not.
    fn to_node(&self, id_mapper: impl FnMut(egg::Id, usize) -> Signal) -> Option<Node<Self::Gate>>;
    fn gate_function(&self) -> Option<GateFunction>;

    /// Returns true iff this node is a not.
    fn is_not(&self) -> bool;
    /// Returns true iff this node is the false constant
    fn is_false(&self) -> bool;
    /// Returns true iff this node is an input
    fn is_input(&self) -> bool;
    /// Returns the input id of this node.
    fn input_id(&self) -> Option<u32>;
    /// Creates a new not node with the given child id.
    fn not(id: egg::Id) -> Self;
}
