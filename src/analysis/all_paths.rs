use std::collections::VecDeque;

use fixedbitset::FixedBitSet;

use crate::graph::lengauer_tarjan::GraphOps;
use crate::graph::v8_heap_graph::{Edge, EdgeId, V8HeapGraph};
use crate::types::NodeId;

#[derive(Clone, Debug, Default)]
pub struct RootPath(Vec<EdgeId>);

impl RootPath {
    pub fn edges<'a>(&'a self, graph: &'a V8HeapGraph) -> impl Iterator<Item = Edge<'a>> {
        self.0.iter().map(|&e| graph.edge(e))
    }
}

pub struct RootPaths {
    paths: Vec<Vec<EdgeId>>,
}

impl RootPaths {
    /// Returns a list of all root paths for the given node
    pub fn paths_to(&self, node: NodeId, graph: &V8HeapGraph) -> Vec<RootPath> {
        if node == 0 {
            return vec![RootPath::default()];
        }

        // For now, combinatorial explosion
        let mut ret: Vec<RootPath> = vec![];
        for &segment in &self.paths[node as usize] {
            let from_node = graph.edge(segment).from_node();
            let mut parent_paths = self.paths_to(from_node, graph);
            for path in &mut parent_paths {
                path.0.push(segment);
            }
            ret.append(&mut parent_paths);
        }
        ret
    }
}

pub fn find_root_paths(graph: &V8HeapGraph, root: NodeId) -> RootPaths {
    let mut paths: Vec<Vec<EdgeId>> = vec![vec![]; graph.node_count()];
    let mut queue = VecDeque::<NodeId>::new();
    let mut seen = FixedBitSet::with_capacity(graph.node_count());

    // Root has an empty path
    queue.push_back(root);
    seen.put(0);
    while let Some(from_node) = queue.pop_front() {
        for edge in graph.out_edges(from_node) {
            if !seen.put(edge.to_node() as usize) {
                paths[edge.to_node() as usize].push(edge.id);
                queue.push_back(edge.to_node());
            }
        }
    }

    RootPaths { paths }
}
