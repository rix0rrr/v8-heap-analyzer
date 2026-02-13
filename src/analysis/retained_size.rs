use crate::graph::CompactGraph;
use crate::types::NodeId;
use ahash::AHashSet;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Default)]
pub struct RetainedSize {
    pub owned: u64,
    pub shared: u64,
}

/// Calculates owned and shared retained sizes for all objects
pub fn calculate_retained_sizes(graph: &CompactGraph) -> HashMap<NodeId, RetainedSize> {
    let node_count = graph.node_count();
    let mut results = HashMap::new();
    
    // Calculate closure for each node
    let mut closures: HashMap<NodeId, AHashSet<NodeId>> = HashMap::new();
    for node_id in 0..node_count as NodeId {
        closures.insert(node_id, compute_closure(graph, node_id));
    }
    
    // For each node, determine which objects in its closure are owned vs shared
    for node_id in 0..node_count as NodeId {
        let closure = &closures[&node_id];
        let mut owned_size = 0u64;
        let mut total_size = 0u64;
        
        for &obj_id in closure {
            let size = graph.node_size(obj_id).unwrap_or(0) as u64;
            total_size += size;
            
            // Check if obj_id is only reachable from this closure
            if is_only_reachable_from_closure(obj_id, closure, &closures, node_count) {
                owned_size += size;
            }
        }
        
        results.insert(node_id, RetainedSize {
            owned: owned_size,
            shared: total_size - owned_size,
        });
    }
    
    results
}

/// Computes the closure of a node (all reachable objects including itself)
fn compute_closure(graph: &CompactGraph, start: NodeId) -> AHashSet<NodeId> {
    let mut closure = AHashSet::new();
    let mut stack = vec![start];
    
    while let Some(node_id) = stack.pop() {
        if closure.insert(node_id) {
            for edge in graph.edges(node_id) {
                stack.push(edge.target);
            }
        }
    }
    
    closure
}

/// Checks if an object is only reachable from the given closure
fn is_only_reachable_from_closure(
    obj_id: NodeId,
    closure: &AHashSet<NodeId>,
    all_closures: &HashMap<NodeId, AHashSet<NodeId>>,
    node_count: usize,
) -> bool {
    // Check if any node outside the closure can reach obj_id
    for node_id in 0..node_count as NodeId {
        if !closure.contains(&node_id) {
            if all_closures[&node_id].contains(&obj_id) {
                return false; // Found a node outside closure that can reach obj_id
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::StringTable;
    use std::sync::Arc;

    #[test]
    fn test_retained_sizes() {
        // Create a simple graph: A -> B -> C, D -> C
        let strings = vec!["".to_string(), "A".to_string(), "B".to_string(), "C".to_string(), "D".to_string()];
        let string_table = Arc::new(StringTable::new(strings));
        let mut graph = CompactGraph::new(string_table);
        
        // Add nodes: A(0), B(1), C(2), D(3)
        graph.node_types.extend(&[3, 3, 3, 3]);
        graph.node_names.extend(&[1, 2, 3, 4]);
        graph.node_ids.extend(&[0, 1, 2, 3]);
        graph.node_sizes.extend(&[100, 50, 30, 40]);
        
        // Edges: A->B, B->C, D->C
        graph.node_edge_ranges.extend(&[(0, 1), (1, 2), (2, 2), (2, 3)]);
        graph.edge_types.extend(&[2, 2, 2]);
        graph.edge_names.extend(&[1, 1, 1]);
        graph.edge_targets.extend(&[1, 2, 2]);
        
        let sizes = calculate_retained_sizes(&graph);
        
        // A's closure: {A, B, C}
        // - A is only in A's closure: owned
        // - B is only in A's and B's closures, but B is in A's closure, so B is owned by A
        // - C is in A's, B's, and D's closures. D is NOT in A's closure, so C is shared
        assert_eq!(sizes[&0].owned, 150); // A + B
        assert_eq!(sizes[&0].shared, 30);  // C
        
        // B's closure: {B, C}
        // - B is in A's closure (outside B's closure), so B is shared
        // - C is in D's closure (outside B's closure), so C is shared
        assert_eq!(sizes[&1].owned, 0);   // Nothing owned
        assert_eq!(sizes[&1].shared, 80); // B + C
        
        // C's closure: {C}
        // - C is in A's, B's, and D's closures (all outside C's closure), so C is shared
        assert_eq!(sizes[&2].owned, 0);   // Nothing owned
        assert_eq!(sizes[&2].shared, 30); // C itself
        
        // D's closure: {D, C}
        // - D is only in D's closure: owned
        // - C is in A's and B's closures (outside D's closure), so C is shared
        assert_eq!(sizes[&3].owned, 40);  // D
        assert_eq!(sizes[&3].shared, 30); // C
    }
}
