use std::collections::HashMap;

use crate::{graph::v8_heap_graph::V8HeapGraph, types::NodeId};

#[derive(Debug, Default)]
pub struct DominatorNode {
    pub retained_size: usize,
    pub children: Vec<NodeId>,
}

pub struct DominatorTree {
    pub children: HashMap<NodeId, Vec<NodeId>>,
    pub retained_sizes: Vec<usize>,
}

impl DominatorTree {
    pub fn retained_size(&self, node_id: NodeId) -> usize {
        self.retained_sizes[node_id as usize]
    }
}

pub fn tree_from_immediate_dominators(
    immediate_dominators: impl IntoIterator<Item = (NodeId, NodeId)>,
    graph: &V8HeapGraph,
) -> DominatorTree {
    let mut ret = DominatorTree {
        children: Default::default(),
        retained_sizes: vec![0; graph.total_node_count()],
    };

    for (node_id, immediate_dominator) in immediate_dominators {
        let children = ret.children.entry(immediate_dominator).or_default();
        children.push(node_id);
    }

    annotate_retained_sizes(0, &ret.children, &mut ret.retained_sizes, graph);

    ret
}

fn annotate_retained_sizes(
    root: NodeId,
    children: &HashMap<NodeId, Vec<NodeId>>,
    retained_sizes: &mut Vec<usize>,
    graph: &V8HeapGraph,
) -> usize {
    retained_sizes[root as usize] = graph.self_size_for(root);

    if let Some(immediate_children) = children.get(&root) {
        for child in immediate_children {
            retained_sizes[root as usize] +=
                annotate_retained_sizes(*child, children, retained_sizes, graph);
        }
    };

    retained_sizes[root as usize]
}
