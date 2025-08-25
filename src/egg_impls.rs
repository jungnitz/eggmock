use egg::{Analysis, CostFunction, EGraph, Extractor, Language, RecExpr};
use rustc_hash::FxHashMap;

use crate::{Id, NetworkLanguage, Node, Receiver, Signal};

type EggId = egg::Id;

fn egraph_id_for_signal<L: NetworkLanguage, A: Analysis<L>>(
    graph: &mut EGraph<L, A>,
    signal: Signal,
) -> EggId {
    let id = EggId::from(signal.node_id().to_usize());
    if signal.is_inverted() {
        let child = graph.id_to_node(id);
        if child.is_not() {
            child.children()[0]
        } else {
            graph.add(L::not(id))
        }
    } else {
        id
    }
}

impl<L: NetworkLanguage, A: Analysis<L>> Receiver for EGraph<L, A> {
    type Gate = L::Gate;
    type Result = (Self, Vec<EggId>);

    fn create(&mut self, node: Node<Self::Gate>) -> Signal {
        let node = L::from_node(node, |signal, _| egraph_id_for_signal(self, signal));
        Signal::new(Id::from_usize(self.add(node).into()), false)
    }

    fn done(mut self, outputs: Vec<Signal>) -> Self::Result {
        let outputs = Vec::from_iter(
            outputs
                .iter()
                .map(|signal| egraph_id_for_signal(&mut self, *signal)),
        );
        (self, outputs)
    }
}

pub trait EggExt {
    type Language: NetworkLanguage;

    fn get_node(&self, id: EggId) -> &Self::Language;

    fn collapse_nots(&self, start_id: EggId) -> (EggId, &Self::Language, bool) {
        let mut id = start_id;
        let mut invert = false;
        loop {
            let node = &self.get_node(id);
            if node.is_not() {
                invert = !invert;
                id = node.children()[0];
            } else {
                break (id, node, invert);
            }
            assert_ne!(id, start_id, "loop detected")
        }
    }

    fn send<R: Receiver<Gate = <Self::Language as NetworkLanguage>::Gate>>(
        &self,
        mut receiver: R,
        outputs: impl IntoIterator<Item = EggId>,
    ) -> R::Result {
        let mut src_to_dest: FxHashMap<EggId, Signal> = FxHashMap::default();
        let mut output_signals = Vec::new();
        for output_id in outputs {
            // depth-first traversal of the underlying expression
            let mut path = Vec::new();

            let mut node_id = output_id;
            let mut node = self.get_node(node_id);
            let mut known_inputs = 0;
            loop {
                if known_inputs == node.children().len() || src_to_dest.contains_key(&node_id) {
                    if known_inputs == node.children().len() {
                        if let Some(dest_node) = node.to_node(|id, _| src_to_dest[&id]) {
                            src_to_dest.insert(node_id, receiver.create(dest_node));
                        } else {
                            // node is a not
                            let child = src_to_dest[&node.children()[0]];
                            src_to_dest.insert(node_id, !child);
                        }
                    }
                    if path.is_empty() {
                        break;
                    }
                    (node_id, node, known_inputs) = path.pop().unwrap();
                    known_inputs += 1;
                } else {
                    let child_id = node.children()[known_inputs];
                    path.push((node_id, node, known_inputs));
                    node_id = child_id;
                    node = self.get_node(node_id);
                    known_inputs = 0;
                }
            }
            output_signals.push(src_to_dest[&output_id]);
        }
        receiver.done(output_signals)
    }
}

impl<'a, CF: CostFunction<L>, L: NetworkLanguage, N: Analysis<L>> EggExt
    for Extractor<'a, CF, L, N>
{
    type Language = L;

    fn get_node(&self, id: EggId) -> &Self::Language {
        self.find_best_node(id)
    }
}

impl<L: NetworkLanguage> EggExt for RecExpr<L> {
    type Language = L;

    fn get_node(&self, id: EggId) -> &Self::Language {
        &self[id]
    }
}
