use std::fmt::Debug;
use std::hash::Hash;

use crate::{Id, Signal};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Node<G> {
    False,
    Input(u32),
    Gate(G),
}

impl<G: Gate> Node<G> {
    pub fn inputs(&self) -> &[Signal] {
        match self {
            Self::False | Self::Input(_) => &[],
            Self::Gate(gate) => gate.inputs(),
        }
    }
    pub fn map_input_signals(self, map: impl FnMut(Signal, usize) -> Signal) -> Self {
        match self {
            Self::False | Self::Input(_) => self,
            Self::Gate(gate) => Node::Gate(gate.map_input_signals(map)),
        }
    }
    pub fn map_input_ids(self, map: impl FnMut(Id, usize) -> Signal) -> Self {
        match self {
            Self::False | Self::Input(_) => self,
            Self::Gate(gate) => Node::Gate(gate.map_input_ids(map)),
        }
    }
}

/// Describes a gate of a logic network.
pub trait Gate: 'static + Debug + Sized + Clone + Hash + Eq {
    /// Returns the same type of gate but with the input signals mapped with the given function.
    fn map_input_signals(self, map: impl FnMut(Signal, usize) -> Signal) -> Self;

    /// Returns the input signals of this gate.
    fn inputs(&self) -> &[Signal];

    /// Returns the same type of gate but with the ids of each input signal replaced by the signal
    /// given by the mapping function. See also [`Signal::map_id`].
    fn map_input_ids(self, mut map: impl FnMut(Id, usize) -> Signal) -> Self {
        self.map_input_signals(|signal, idx| signal.map_id(|id| map(id, idx)))
    }

    fn function(&self) -> GateFunction;
}

/// Function of a gate.
pub enum GateFunction {
    And,
    Or,
    Xor,
    Maj,
}
