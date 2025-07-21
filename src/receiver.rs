use std::marker::PhantomData;

use crate::Node;

use super::Signal;

/// A type that receives a logic network and produce a result from it.
pub trait Receiver: Sized {
    type Gate;
    type Result;

    fn create_gate(&mut self, gate: Self::Gate) -> Signal {
        self.create(Node::Gate(gate))
    }

    fn create(&mut self, node: Node<Self::Gate>) -> Signal;

    /// Creates the result from the previously transferred nodes where `outputs` contains the output
    /// signals.
    fn done(self, outputs: Vec<Signal>) -> Self::Result;

    /// Maps the result of this Receiver using the given function.
    fn map<NewResult, F>(self, map: F) -> impl Receiver<Gate = Self::Gate, Result = NewResult>
    where
        F: FnOnce(Self::Result) -> NewResult,
    {
        MappedReceiver {
            original: self,
            map,
        }
    }

    fn adapt<From, A: FnMut(&mut Self, From) -> Signal>(
        self,
        adapter: A,
    ) -> impl Receiver<Gate = From, Result = Self::Result> {
        AdaptedReceiver {
            _from: PhantomData,
            to: self,
            adapter,
        }
    }

    fn with_input<G: ReceiveInto<Self::Gate>>(
        self,
    ) -> impl Receiver<Gate = G, Result = Self::Result> {
        ReceiveIntoReceiver {
            _gate: PhantomData,
            original: self,
        }
    }
}

pub trait ReceiveInto<Gate> {
    fn receive_into(self, receiver: &mut impl Receiver<Gate = Gate>) -> Signal;
}

pub trait ReceiveFrom<Gate> {
    fn receive_from(from: Gate, receiver: &mut impl Receiver<Gate = Self>) -> Signal;
}

impl<From, Into> ReceiveInto<Into> for From
where
    Into: ReceiveFrom<From>,
{
    fn receive_into(self, receiver: &mut impl Receiver<Gate = Into>) -> Signal {
        Into::receive_from(self, receiver)
    }
}

struct ReceiveIntoReceiver<G, O> {
    _gate: PhantomData<G>,
    original: O,
}

impl<G, O> Receiver for ReceiveIntoReceiver<G, O>
where
    O: Receiver,
    G: ReceiveInto<O::Gate>,
{
    type Gate = G;
    type Result = O::Result;

    fn create(&mut self, node: Node<Self::Gate>) -> Signal {
        match node {
            Node::False => self.original.create(Node::False),
            Node::Input(idx) => self.original.create(Node::Input(idx)),
            Node::Gate(gate) => gate.receive_into(&mut self.original),
        }
    }

    fn done(self, outputs: Vec<Signal>) -> Self::Result {
        self.original.done(outputs)
    }
}

struct MappedReceiver<Original, Function> {
    original: Original,
    map: Function,
}

impl<O, R, F> Receiver for MappedReceiver<O, F>
where
    O: Receiver,
    F: FnOnce(O::Result) -> R,
{
    type Gate = O::Gate;
    type Result = R;

    fn create(&mut self, node: Node<Self::Gate>) -> Signal {
        self.original.create(node)
    }

    fn done(self, outputs: Vec<Signal>) -> Self::Result {
        (self.map)(self.original.done(outputs))
    }
}

struct AdaptedReceiver<From, To, F> {
    _from: PhantomData<fn(From) -> ()>,
    to: To,
    adapter: F,
}

impl<From, To, F> Receiver for AdaptedReceiver<From, To, F>
where
    To: Receiver,
    F: FnMut(&mut To, From) -> Signal,
{
    type Gate = From;
    type Result = To::Result;

    fn create(&mut self, node: Node<Self::Gate>) -> Signal {
        match node {
            Node::False => self.to.create(Node::False),
            Node::Input(idx) => self.to.create(Node::Input(idx)),
            Node::Gate(gate) => (self.adapter)(&mut self.to, gate),
        }
    }

    fn done(self, outputs: Vec<Signal>) -> Self::Result {
        self.to.done(outputs)
    }
}
