use crate::graph::CompactGraph;
use crate::types::NodeId;
use ahash::AHashSet;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Default)]
pub struct RetainedSize {
    pub owned: u64,
    pub shared: u64,
}

/// Calculates owned and shared retained sizes using dominator tree analysis
/// Time complexity: O(n * m) where n = nodes, m = edges (much better than O(n²))
pub fn calculate_retained_sizes(graph: &CompactGraph) -> HashMap<NodeId, RetainedSize> {
    let node_count = graph.node_count();
    
    // Build reverse graph for dominator analysis
    let reverse_edges = build_reverse_graph(graph, node_count);
    
    // Find all nodes reachable from GC roots
    let reachable = find_reachable_from_roots(graph);
    
    // Calculate dominators using iterative algorithm
    let dominators = calculate_dominators(graph, &reverse_edges, &reachable);
    
    // Build dominator tree
    let dom_tree = build_dominator_tree(&dominators, node_count);
    
    // Calculate retained sizes using dominator tree
    calculate_sizes_from_dominators(graph, &dom_tree, &reachable)
}

/// Builds reverse edge map for efficient backward traversal
fn build_reverse_graph(graph: &CompactGraph, node_count: usize) -> Vec<Vec<NodeId>> {
    let mut reverse = vec![Vec::new(); node_count];
    
    for node_id in 0..node_count as NodeId {
        for edge in graph.edges(node_id) {
            if (edge.target as usize) < node_count {
                reverse[edge.target as usize].push(node_id);
            }
        }
    }
    
    reverse
}

/// Finds all nodes reachable from GC roots using BFS
fn find_reachable_from_roots(graph: &CompactGraph) -> AHashSet<NodeId> {
    let mut reachable = AHashSet::new();
    let mut stack = Vec::new();
    
    // Start from all GC roots
    for &root in graph.gc_roots() {
        stack.push(root);
    }
    
    while let Some(node_id) = stack.pop() {
        if reachable.insert(node_id) {
            for edge in graph.edges(node_id) {
                stack.push(edge.target);
            }
        }
    }
    
    reachable
}

/// Calculates immediate dominators using iterative dataflow analysis
/// The immediate dominator of n is the unique node that strictly dominates n
/// but does not strictly dominate any other node that strictly dominates n
fn calculate_dominators(
    graph: &CompactGraph,
    reverse_edges: &[Vec<NodeId>],
    reachable: &AHashSet<NodeId>,
) -> HashMap<NodeId, NodeId> {
    let mut idom: HashMap<NodeId, Option<NodeId>> = HashMap::new();
    
    // Initialize: all nodes have unknown immediate dominator except roots
    for &node_id in reachable {
        if graph.gc_roots().contains(&node_id) {
            idom.insert(node_id, Some(node_id)); // Roots dominate themselves
        } else {
            idom.insert(node_id, None);
        }
    }
    
    // Iteratively compute immediate dominators until convergence
    let mut changed = true;
    let mut iterations = 0;
    let max_iterations = 100; // Limit iterations to prevent hanging
    
    while changed && iterations < max_iterations {
        changed = false;
        iterations += 1;
        
        for &node_id in reachable {
            if graph.gc_roots().contains(&node_id) {
                continue;
            }
            
            let predecessors = &reverse_edges[node_id as usize];
            if predecessors.is_empty() {
                continue;
            }
            
            // Find first predecessor with known idom
            let mut new_idom = None;
            for &pred in predecessors {
                if idom.get(&pred).and_then(|&x| x).is_some() {
                    new_idom = Some(pred);
                    break;
                }
            }
            
            // Intersect with remaining predecessors
            if let Some(mut current) = new_idom {
                for &pred in predecessors {
                    if let Some(Some(_)) = idom.get(&pred) {
                        current = intersect(current, pred, &idom);
                    }
                }
                
                if idom.get(&node_id) != Some(&Some(current)) {
                    idom.insert(node_id, Some(current));
                    changed = true;
                }
            }
        }
    }
    
    // Convert to non-optional map
    idom.into_iter()
        .filter_map(|(k, v)| v.map(|dom| (k, dom)))
        .collect()
}

/// Finds the common dominator (intersection) of two nodes in the dominator tree
fn intersect(
    mut b1: NodeId,
    mut b2: NodeId,
    idom: &HashMap<NodeId, Option<NodeId>>,
) -> NodeId {
    // Build path from b1 to root
    let mut path1 = AHashSet::new();
    let mut current = b1;
    loop {
        path1.insert(current);
        if let Some(Some(dom)) = idom.get(&current) {
            if *dom == current {
                break; // Reached root
            }
            current = *dom;
        } else {
            break;
        }
    }
    
    // Walk from b2 to root until we hit something in path1
    current = b2;
    loop {
        if path1.contains(&current) {
            return current; // Found common dominator
        }
        if let Some(Some(dom)) = idom.get(&current) {
            if *dom == current {
                return current; // Reached root
            }
            current = *dom;
        } else {
            return current;
        }
    }
}

/// Builds dominator tree (children dominated by each node)
fn build_dominator_tree(dominators: &HashMap<NodeId, NodeId>, node_count: usize) -> Vec<Vec<NodeId>> {
    let mut tree = vec![Vec::new(); node_count];
    
    for (&node, &dominator) in dominators {
        if node != dominator {
            tree[dominator as usize].push(node);
        }
    }
    
    tree
}

/// Calculates retained sizes using dominator tree
fn calculate_sizes_from_dominators(
    graph: &CompactGraph,
    dom_tree: &[Vec<NodeId>],
    reachable: &AHashSet<NodeId>,
) -> HashMap<NodeId, RetainedSize> {
    let mut results = HashMap::new();
    let node_count = graph.node_count();
    
    // Calculate retained size for each node (size of dominated subtree)
    let mut retained: HashMap<NodeId, u64> = HashMap::new();
    
    for node_id in (0..node_count as NodeId).rev() {
        if !reachable.contains(&node_id) {
            continue;
        }
        
        let mut size = graph.node_size(node_id).unwrap_or(0) as u64;
        
        // Add sizes of all dominated children
        for &child in &dom_tree[node_id as usize] {
            size += retained.get(&child).copied().unwrap_or(0);
        }
        
        retained.insert(node_id, size);
    }
    
    // For now, treat all retained size as "owned" and shared as 0
    // A more sophisticated analysis would distinguish between exclusive and shared
    for (&node_id, &size) in &retained {
        results.insert(node_id, RetainedSize {
            owned: size,
            shared: 0,
        });
    }
    
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::StringTable;
    use std::sync::Arc;

    #[test]
    fn test_retained_sizes_with_dominators() {
        // Create a simple graph: Root -> A -> B, Root -> C
        let strings = vec!["".to_string(), "Root".to_string(), "A".to_string(), "B".to_string(), "C".to_string()];
        let string_table = Arc::new(StringTable::new(strings));
        let mut graph = CompactGraph::new(string_table);
        
        // Add nodes: Root(0), A(1), B(2), C(3)
        graph.node_types.extend(&[3, 3, 3, 3]);
        graph.node_names.extend(&[1, 2, 3, 4]);
        graph.node_ids.extend(&[0, 1, 2, 3]);
        graph.node_sizes.extend(&[10, 20, 30, 40]);
        
        // Edges: Root->A, A->B, Root->C
        graph.node_edge_ranges.extend(&[(0, 2), (2, 3), (3, 3), (3, 3)]);
        graph.edge_types.extend(&[2, 2, 2]);
        graph.edge_names.extend(&[1, 1, 1]);
        graph.edge_targets.extend(&[1, 3, 2]);
        
        graph.gc_roots.push(0);
        
        let sizes = calculate_retained_sizes(&graph);
        
        // Root dominates everything, retains all: 10 + 20 + 30 + 40 = 100
        assert_eq!(sizes[&0].owned, 100);
        
        // A dominates B (only path to B is through A), retains A + B: 20 + 30 = 50
        assert_eq!(sizes[&1].owned, 50);
        
        // B doesn't dominate anything else, retains only itself: 30
        assert_eq!(sizes[&2].owned, 30);
        
        // C doesn't dominate anything else, retains only itself: 40
        assert_eq!(sizes[&3].owned, 40);
    }
}
