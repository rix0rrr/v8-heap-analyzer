use crate::{analysis::lengauer_tarjan::GraphOps, graph::v8_heap_graph::V8HeapGraph};

impl GraphOps for V8HeapGraph {
    fn node_count(&self) -> usize {
        self.node_count()
    }

    fn predecessors(&self, node: crate::types::NodeId) -> &[crate::types::NodeId] {
        self.in_edges(node)
    }

    fn successors(&self, node: crate::types::NodeId) -> &[crate::types::NodeId] {
        self.out_edges(node)
    }
}
