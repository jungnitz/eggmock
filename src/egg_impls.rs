use egg::{Analysis, CostFunction, EGraph, Extractor, Language};
use std::ops::Index;

use crate::{Id, Network, NetworkLanguage, Node, Receiver, Signal};

fn egraph_id_for_signal<L: NetworkLanguage, A: Analysis<L>>(
    graph: &mut EGraph<L, A>,
    signal: Signal,
) -> egg::Id {
    let child_id = signal.node_id().into();
    if signal.is_inverted() {
        let child = graph.id_to_node(child_id);
        if child.is_not() {
            child.children()[0]
        } else {
            graph.add(L::not(child_id))
        }
    } else {
        child_id
    }
}

fn create_node<L: NetworkLanguage, A: Analysis<L>>(
    graph: &mut EGraph<L, A>,
    node: Node<L::Gate>,
) -> Signal {
    let node = L::from_node(node, |signal| egraph_id_for_signal(graph, signal));
    Signal::new(Id::from(graph.add(node)), false)
}

impl<L: NetworkLanguage, A: Analysis<L>> Receiver for EGraph<L, A> {
    type Gate = L::Gate;
    type Result = (Self, Vec<egg::Id>);

    fn create_input(&mut self, idx: u32) -> Signal {
        create_node(self, Node::Input(idx))
    }

    fn create_false(&mut self) -> Signal {
        create_node(self, Node::False)
    }

    fn create(&mut self, gate: L::Gate) -> Signal {
        create_node(self, Node::Gate(gate))
    }

    fn done(mut self, outputs: &[Signal]) -> Self::Result {
        let outputs = Vec::from_iter(
            outputs
                .iter()
                .map(|signal| egraph_id_for_signal(&mut self, *signal)),
        );
        (self, outputs)
    }
}

impl<L: NetworkLanguage, CF: CostFunction<L>, A: Analysis<L>> Network
    for (Extractor<'_, CF, L, A>, Vec<egg::Id>)
{
    type Gate = L::Gate;

    fn outputs(&self) -> impl Iterator<Item = Signal> {
        self.1
            .iter()
            .map(|o| ExtractorIndexWrapper(&self.0).to_signal(*o))
    }

    fn node(&self, id: Id) -> Node<Self::Gate> {
        self.0
            .find_best_node(id.into())
            .to_node(|id| ExtractorIndexWrapper(&self.0).to_signal(id))
            .expect("id should point to a non-not node")
    }
}

pub trait EggIdToSignal {
    fn to_signal(&self, id: egg::Id) -> Signal;
}

impl<I: Index<egg::Id, Output: NetworkLanguage>> EggIdToSignal for I {
    fn to_signal(&self, start_id: egg::Id) -> Signal {
        let mut id = start_id;
        let mut invert = false;
        loop {
            let node = &self[id];
            if node.is_not() {
                invert = !invert;
                id = node.children()[0];
            } else {
                break Signal::new(id.into(), invert);
            }
            assert_ne!(id, start_id, "loop detected")
        }
    }
}

struct ExtractorIndexWrapper<'r, E>(&'r E);

impl<CF, L, A> Index<egg::Id> for ExtractorIndexWrapper<'_, Extractor<'_, CF, L, A>>
where
    CF: CostFunction<L>,
    L: NetworkLanguage,
    A: Analysis<L>,
{
    type Output = L;

    fn index(&self, index: egg::Id) -> &Self::Output {
        self.0.find_best_node(index)
    }
}
