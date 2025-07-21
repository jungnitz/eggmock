use eggmock::{
    EggExt, Mig, MigLanguage, Receiver, ReceiverFFI, Rewriter, RewriterFFI,
    egg::{CostFunction, EGraph, Extractor, Id, Language, Runner, rewrite},
};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::rc::Rc;
use std::time::Duration;

/// Optimizes for number of constant-uses
struct ExampleCostFunction;

#[derive(Debug, Clone)]
struct ExampleCost(Rc<HashSet<MigLanguage>>);

impl PartialEq<Self> for ExampleCost {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
    }
}

impl PartialOrd for ExampleCost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.len().partial_cmp(&other.0.len())
    }
}

impl CostFunction<MigLanguage> for ExampleCostFunction {
    type Cost = ExampleCost;

    fn cost<C>(&mut self, enode: &MigLanguage, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        if let &MigLanguage::False = enode {
            return ExampleCost(Rc::new(HashSet::from([enode.clone()])));
        }
        let mut set = HashSet::new();
        for child in enode.children() {
            for r in costs(*child).0.iter() {
                if let MigLanguage::False = r {
                    set.insert(enode.clone());
                } else {
                    set.insert(r.clone());
                }
            }
        }
        ExampleCost(Rc::new(set))
    }
}

struct ExampleRewriter;

impl Rewriter for ExampleRewriter {
    type Gate = Mig;
    type Intermediate = (EGraph<MigLanguage, ()>, Vec<Id>);

    fn create_receiver(
        &mut self,
    ) -> impl Receiver<Gate = Mig, Result = Self::Intermediate> + 'static {
        EGraph::new(())
    }

    fn rewrite(
        self,
        (graph, roots): Self::Intermediate,
        output: impl Receiver<Gate = Mig, Result = ()>,
    ) {
        let rules = &[
            rewrite!("commute_1"; "(maj ?a ?b ?c)" => "(maj ?b ?a ?c)"),
            rewrite!("commute_2"; "(maj ?a ?b ?c)" => "(maj ?a ?c ?b)"),
            rewrite!("example"; "(maj (! f) ?a (maj f ?b ?c))" => "(maj (! f) ?a (maj ?a ?b ?c))"),
        ];
        let runner = Runner::default()
            .with_time_limit(Duration::from_secs(60))
            .with_egraph(graph)
            .run(rules);
        runner.print_report();
        Extractor::new(&runner.egraph, ExampleCostFunction).send(output, roots);
    }
}

#[unsafe(no_mangle)]
extern "C" fn example_mig_rewrite() -> ReceiverFFI<'static, RewriterFFI> {
    RewriterFFI::new(ExampleRewriter)
}
