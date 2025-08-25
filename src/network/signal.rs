use std::{
    fmt::Debug,
    ops::{BitXor, Not},
};

use crate::Id;

/// References a node by its id with a flag that indicates whether the signal from this node is
/// inverted.
///
/// Internally, this is represented as a `u32` where the MSB indicates whether it is inverted.
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
#[repr(C)]
pub struct Signal(u32);

impl Signal {
    const NOT_MASK: u32 = 1 << 31;

    pub fn new(id: Id, inverted: bool) -> Signal {
        Signal(id.0).maybe_invert(inverted)
    }

    pub fn is_inverted(&self) -> bool {
        self.0 & Self::NOT_MASK != 0
    }
    pub fn maybe_invert(&self, invert: bool) -> Signal {
        Signal(self.0 ^ ((invert as u32) << 31))
    }
    pub fn invert(&self) -> Signal {
        self.maybe_invert(true)
    }
    pub fn node_id(&self) -> Id {
        Id::from(self.0 & !Self::NOT_MASK)
    }
    /// Replaces the id of this signal with the given signal. That is, the id of the returned signal
    /// is the same as the id of the parameter signal and the returned signal will be inverted if
    /// exactly one of the two given signals is inverted.
    pub fn replace_id(&self, signal: Signal) -> Signal {
        Signal::new(signal.node_id(), self.is_inverted() ^ signal.is_inverted())
    }
    /// Performs [`Self::replace_id`] with the signal given by the mapping function for this
    /// signal's id.
    pub fn map_id(&self, map: impl FnOnce(Id) -> Signal) -> Signal {
        self.replace_id(map(self.node_id()))
    }
}

impl Debug for Signal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Signal(")?;
        if self.is_inverted() {
            write!(f, "!{})", self.node_id().0)
        } else {
            write!(f, "{})", self.node_id().0)
        }
    }
}

impl Not for Signal {
    type Output = Self;

    fn not(self) -> Self::Output {
        self.invert()
    }
}

impl BitXor<bool> for Signal {
    type Output = Signal;

    fn bitxor(self, rhs: bool) -> Self::Output {
        self.maybe_invert(rhs)
    }
}
