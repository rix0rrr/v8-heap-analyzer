use std::collections::HashMap;

use itertools::Itertools;

use crate::{
    graph::v8_heap_graph::{Node, V8HeapGraph},
    types::NodeId,
    utils::format_bytes,
};

#[derive(Debug, Default)]
pub struct DominatorNode {
    pub retained_size: usize,
    pub children: Vec<NodeId>,
}

pub struct DominatorTree {
    children: HashMap<NodeId, Vec<NodeId>>,
    retained_sizes: Vec<usize>,
}

impl DominatorTree {
    pub fn retained_size(&self, node_id: NodeId) -> usize {
        self.retained_sizes[node_id as usize]
    }
}

pub fn tree_from_immediate_dominators<'a>(
    immediate_dominators: impl IntoIterator<Item = (NodeId, NodeId)>,
    graph: &V8HeapGraph,
) -> DominatorTree {
    let mut ret = DominatorTree {
        children: Default::default(),
        retained_sizes: vec![0; graph.node_count()],
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

pub fn print_dominator_tree(tree: &DominatorTree, graph: &V8HeapGraph) {
    print_dominator_node(0, tree, graph, 0);
}

fn print_dominator_node(node_id: NodeId, tree: &DominatorTree, graph: &V8HeapGraph, depth: usize) {
    let node = graph.node(node_id);
    let retained_size = tree.retained_sizes[node_id as usize];

    println!(
        "{}({}) {}@{} ({}) {}",
        "    ".repeat(depth),
        node.typ_str(),
        node.print_safe_name(40),
        node.id.clone(),
        format_bytes(retained_size),
        show_node(node)
    );

    if let Some(mut children) = tree.children.get(&node_id).cloned() {
        // Sort by retained sizes descending
        children.sort_by_key(|node| -(tree.retained_sizes[*node as usize] as i64));

        for child in &children[0..20.min(children.len())] {
            print_dominator_node(*child, tree, graph, depth + 1);
        }
    }
}

fn show_node(node: Node<'_>) -> String {
    node.graph
        .edges_for(node.id)
        .map(|e| format!("{} {} {}", e.typ_str(), e.name_or_index(), e.to_node()))
        .join(", ")
}
