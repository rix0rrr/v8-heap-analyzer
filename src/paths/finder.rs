use crate::graph::CompactGraph;
use crate::types::NodeId;
use std::collections::{HashMap, VecDeque};

pub struct RetentionPathFinder<'a> {
    graph: &'a CompactGraph,
}

#[derive(Debug, Clone)]
pub struct RetentionPath {
    pub length: usize,
    pub nodes: Vec<NodeId>,
    pub edge_types: Vec<u8>,
    pub edge_names: Vec<String>,
}

impl<'a> RetentionPathFinder<'a> {
    pub fn new(graph: &'a CompactGraph) -> Self {
        Self { graph }
    }

    pub fn find_paths(&self, target: NodeId, max_paths: usize) -> Vec<RetentionPath> {
        let mut paths = Vec::new();
        let mut visited = HashMap::new();
        let mut queue = VecDeque::new();
        
        // Start from all GC roots
        for &root in self.graph.gc_roots() {
            queue.push_back(root);
            visited.insert(root, (None, 0u8, String::new()));
        }
        
        // BFS to find paths
        while let Some(current) = queue.pop_front() {
            if current == target {
                // Found target, reconstruct path
                let path = self.reconstruct_path(&visited, target);
                paths.push(path);
                
                if paths.len() >= max_paths {
                    break;
                }
                continue;
            }
            
            // Explore edges
            for edge in self.graph.edges(current) {
                if !visited.contains_key(&edge.target) {
                    let edge_name = edge.name().unwrap_or("").to_string();
                    visited.insert(edge.target, (Some(current), edge.edge_type, edge_name));
                    queue.push_back(edge.target);
                }
            }
        }
        
        paths
    }

    fn reconstruct_path(&self, visited: &HashMap<NodeId, (Option<NodeId>, u8, String)>, target: NodeId) -> RetentionPath {
        let mut nodes = Vec::new();
        let mut edge_types = Vec::new();
        let mut edge_names = Vec::new();
        
        let mut current = target;
        nodes.push(current);
        
        while let Some(&(parent_opt, edge_type, ref edge_name)) = visited.get(&current) {
            if let Some(parent) = parent_opt {
                edge_types.push(edge_type);
                edge_names.push(edge_name.clone());
                nodes.push(parent);
                current = parent;
            } else {
                break;
            }
        }
        
        // Reverse to get path from root to target
        nodes.reverse();
        edge_types.reverse();
        edge_names.reverse();
        
        RetentionPath {
            length: nodes.len(),
            nodes,
            edge_types,
            edge_names,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::StringTable;
    use std::sync::Arc;

    #[test]
    fn test_find_retention_path() {
        let strings = vec![
            "".to_string(),
            "Window".to_string(),
            "document".to_string(),
            "cache".to_string(),
        ];
        let string_table = Arc::new(StringTable::new(strings));
        
        let mut graph = CompactGraph::new(string_table);
        
        // Node 0: Window (GC root)
        // Node 1: document
        // Node 2: cache
        graph.node_types.extend(&[3, 3, 3]);
        graph.node_names.extend(&[1, 2, 3]);
        graph.node_ids.extend(&[1, 2, 3]);
        graph.node_sizes.extend(&[100, 200, 50]);
        graph.node_edge_ranges.extend(&[(0, 1), (1, 2), (2, 2)]);
        graph.gc_roots.push(0);
        
        // Edges: 0 -> 1 -> 2
        graph.edge_types.extend(&[2, 2]);
        graph.edge_names.extend(&[2, 3]);
        graph.edge_targets.extend(&[1, 2]);
        
        let finder = RetentionPathFinder::new(&graph);
        let paths = finder.find_paths(2, 1);
        
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].nodes, vec![0, 1, 2]);
        assert_eq!(paths[0].length, 3);
    }
}
