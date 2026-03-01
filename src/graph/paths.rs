use std::{collections::HashSet, rc::Rc};

use crate::{
    graph::v8_heap_graph::{EdgeId, EdgeType, V8HeapGraph},
    types::NodeId,
};

pub type RootPath = Vec<EdgeId>;

/// Returns a list of all paths between `node` and `relative_to`. Paths are represented as lists of edge ids, starting from `relative_to` and ending at `node`.
pub fn paths_between(node: NodeId, relative_to: NodeId, graph: &V8HeapGraph) -> Vec<RootPath> {
    if let Some(tree) = path_tree(node, relative_to, graph) {
        paths_from_tree(&tree)
    } else {
        vec![]
    }
}

#[derive(Debug)]
struct PathTree {
    incoming: Vec<(EdgeId, Rc<PathTree>)>,
}

fn path_tree(node: NodeId, relative_to: NodeId, graph: &V8HeapGraph) -> Option<Rc<PathTree>> {
    path_tree_rec(node, relative_to, graph, &mut Default::default())
}

fn path_tree_rec(
    node: NodeId,
    relative_to: NodeId,
    graph: &V8HeapGraph,
    recursion_breaker: &mut HashSet<NodeId>,
) -> Option<Rc<PathTree>> {
    if node == relative_to {
        return Some(Rc::new(PathTree { incoming: vec![] }));
    }

    if recursion_breaker.contains(&node) {
        return None;
    }
    recursion_breaker.insert(node);

    let mut incoming = vec![];
    for edge in graph.in_edges(node) {
        // Skip weak and shortcut edges
        if matches!(edge.typ(), EdgeType::Weak | EdgeType::Shortcut) {
            continue;
        }

        if let Some(path) = path_tree_rec(edge.from_node(), relative_to, graph, recursion_breaker) {
            incoming.push((edge.id, path));
        }

        if recursion_breaker.len() > 100 {
            break;
        }
    }
    if incoming.is_empty() {
        None
    } else {
        Some(Rc::new(PathTree { incoming }))
    }
}

fn paths_from_tree(tree: &PathTree) -> Vec<RootPath> {
    if tree.incoming.is_empty() {
        return vec![vec![]];
    }

    let mut ret = vec![];
    for (edge_id, subtree) in &tree.incoming {
        for mut path in paths_from_tree(subtree) {
            path.push(*edge_id);
            ret.push(path);
        }
    }
    ret
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_paths_from_tree() {
        use super::*;

        let tree = PathTree {
            incoming: vec![
                (EdgeId::new(1), Rc::new(PathTree { incoming: vec![] })),
                (
                    EdgeId::new(2),
                    Rc::new(PathTree {
                        incoming: vec![
                            (EdgeId::new(3), Rc::new(PathTree { incoming: vec![] })),
                            (EdgeId::new(4), Rc::new(PathTree { incoming: vec![] })),
                        ],
                    }),
                ),
            ],
        };

        let paths = paths_from_tree(&tree);
        assert!(paths.contains(&vec![EdgeId::new(1)]));
        assert!(paths.contains(&vec![EdgeId::new(3), EdgeId::new(2)]));
        assert!(paths.contains(&vec![EdgeId::new(4), EdgeId::new(2)]));
    }
}
