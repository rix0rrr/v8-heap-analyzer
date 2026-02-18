use crate::graph::{
    lengauer_tarjan::IterWrapper,
    v8_heap_graph::{Edge, EdgeType},
};

impl<'a> super::lengauer_tarjan::GraphOps<'a> for super::v8_heap_graph::V8HeapGraph {
    type PredIter = IterWrapper<'a>;
    type SuccIter = IterWrapper<'a>;

    fn node_count(&self) -> usize {
        self.total_node_count()
    }

    fn predecessors(&'a self, node: crate::types::NodeId) -> Self::PredIter {
        // This is only used for dominator calculations, and we want to ignore weak nodes there
        IterWrapper::new(self.in_edges(node).filter(no_weak).map(|e| e.from_node()))
    }

    fn successors(&'a self, node: crate::types::NodeId) -> Self::SuccIter {
        // This is only used for dominator calculations, and we want to ignore weak nodes there
        IterWrapper::new(self.out_edges(node).filter(no_weak).map(|e| e.to_node()))
    }
}

fn no_weak<'a>(e: &Edge<'a>) -> bool {
    e.typ() != EdgeType::Weak
}
