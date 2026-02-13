use crate::graph::CompactGraph;
use crate::types::NodeId;
use ahash::{AHashMap, AHashSet};
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct DuplicateAnalyzer {
    graph: CompactGraph,
    include_hidden_classes: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateGroup {
    pub hash: u64,
    pub object_type: String,
    pub count: usize,
    pub size_per_object: u64,  // Changed to u64 for total size
    pub total_wasted: u64,
    pub representative: NodeId,
    pub node_ids: Vec<NodeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_value: Option<String>,
}

impl DuplicateAnalyzer {
    pub fn new(graph: CompactGraph, include_hidden_classes: bool) -> Self {
        Self {
            graph,
            include_hidden_classes,
        }
    }

    pub fn find_duplicates(&self) -> Vec<DuplicateGroup> {
        let mut all_groups = Vec::new();
        
        all_groups.extend(self.find_duplicate_strings());
        all_groups.extend(self.find_duplicate_objects());
        
        // Sort by total wasted memory
        all_groups.sort_by(|a, b| b.total_wasted.cmp(&a.total_wasted));
        
        all_groups
    }

    pub fn find_duplicate_strings(&self) -> Vec<DuplicateGroup> {
        let mut hash_map: AHashMap<u64, Vec<NodeId>> = AHashMap::new();
        
        for node_id in 0..self.graph.node_count() as NodeId {
            let node_type = self.graph.node_type(node_id).unwrap();
            
            // Type 2 is typically string in V8
            if node_type == 2 {
                if let Some(name) = self.graph.node_name(node_id) {
                    let hash = self.hash_string(name);
                    hash_map.entry(hash).or_default().push(node_id);
                }
            }
        }
        
        self.create_groups(hash_map, "String")
    }

    pub fn find_duplicate_objects(&self) -> Vec<DuplicateGroup> {
        let mut hash_map: AHashMap<u64, Vec<NodeId>> = AHashMap::new();
        
        for node_id in 0..self.graph.node_count() as NodeId {
            let node_type = self.graph.node_type(node_id).unwrap();
            
            // Type 3 is typically object in V8
            if node_type == 3 {
                let hash = self.hash_object(node_id);
                hash_map.entry(hash).or_default().push(node_id);
            }
        }
        
        self.create_groups(hash_map, "Object")
    }

    fn hash_string(&self, s: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }

    fn hash_object(&self, node_id: NodeId) -> u64 {
        let mut hasher = DefaultHasher::new();
        
        // Hash object type
        if let Some(name) = self.graph.node_name(node_id) {
            name.hash(&mut hasher);
        }
        
        // Hash properties (edges)
        let mut edges: Vec<_> = self.graph.edges(node_id).collect();
        edges.sort_by_key(|e| e.name_or_index);
        
        for edge in edges {
            if !self.include_hidden_classes && edge.edge_type == 4 {
                continue; // Skip hidden edges
            }
            
            edge.edge_type.hash(&mut hasher);
            edge.name_or_index.hash(&mut hasher);
            edge.target.hash(&mut hasher);
        }
        
        hasher.finish()
    }

    fn calculate_total_size(&self, node_id: NodeId) -> u64 {
        // For now, just return shallow size
        // TODO: Implement proper retained size calculation that only counts
        // objects uniquely owned by this object, not shared references
        self.graph.node_size(node_id).unwrap_or(0) as u64
    }

    fn calculate_size_recursive(&self, node_id: NodeId, visited: &mut AHashSet<NodeId>) -> u64 {
        if visited.contains(&node_id) {
            return 0; // Already counted or circular reference
        }
        visited.insert(node_id);
        
        let mut total = self.graph.node_size(node_id).unwrap_or(0) as u64;
        
        // Add sizes of all referenced objects
        for edge in self.graph.edges(node_id) {
            if !self.include_hidden_classes && edge.edge_type == 4 {
                continue; // Skip hidden edges
            }
            total += self.calculate_size_recursive(edge.target, visited);
        }
        
        total
    }

    fn get_sample_value(&self, node_id: NodeId) -> Option<String> {
        let node_type = self.graph.node_type(node_id)?;
        
        // For strings, return the string value
        if node_type == 2 {
            return self.graph.node_name(node_id).map(|s| {
                if s.len() > 100 {
                    // Truncate at char boundary, not byte boundary
                    let truncated: String = s.chars().take(100).collect();
                    format!("\"{}...\"", truncated)
                } else {
                    format!("\"{}\"", s)
                }
            });
        }
        
        // For objects, show structure
        if node_type == 3 {
            let mut parts = Vec::new();
            let edges: Vec<_> = self.graph.edges(node_id).take(5).collect();
            
            for edge in edges {
                if let Some(name) = edge.name() {
                    if let Some(target_name) = self.graph.node_name(edge.target) {
                        parts.push(format!("{}: {}", name, target_name));
                    }
                }
            }
            
            if parts.is_empty() {
                return Some("{}".to_string());
            }
            
            return Some(format!("{{ {} }}", parts.join(", ")));
        }
        
        None
    }

    fn create_groups(&self, hash_map: AHashMap<u64, Vec<NodeId>>, type_name: &str) -> Vec<DuplicateGroup> {
        let mut groups = Vec::new();
        
        for (hash, node_ids) in hash_map {
            if node_ids.len() > 1 {
                let representative = node_ids[0];
                let size = self.calculate_total_size(representative);
                let count = node_ids.len();
                let total_wasted = (count - 1) as u64 * size;
                
                let object_type = self.graph.node_name(representative)
                    .unwrap_or(type_name)
                    .to_string();
                
                let sample_value = self.get_sample_value(representative);
                
                groups.push(DuplicateGroup {
                    hash,
                    object_type,
                    count,
                    size_per_object: size,
                    total_wasted,
                    representative,
                    node_ids,
                    sample_value,
                });
            }
        }
        
        groups
    }

    pub fn into_graph(self) -> CompactGraph {
        self.graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::StringTable;
    use std::sync::Arc;

    #[test]
    fn test_find_duplicate_strings() {
        let strings = vec![
            "".to_string(),
            "duplicate".to_string(),
            "unique".to_string(),
        ];
        let string_table = Arc::new(StringTable::new(strings));
        
        let mut graph = CompactGraph::new(string_table);
        
        // Add 3 string nodes: 2 duplicates, 1 unique
        graph.node_types.extend(&[2, 2, 2]);
        graph.node_names.extend(&[1, 1, 2]); // Two "duplicate", one "unique"
        graph.node_ids.extend(&[1, 2, 3]);
        graph.node_sizes.extend(&[48, 48, 32]);
        graph.node_edge_ranges.extend(&[(0, 0), (0, 0), (0, 0)]);
        
        let analyzer = DuplicateAnalyzer::new(graph, false);
        let groups = analyzer.find_duplicate_strings();
        
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].count, 2);
        assert_eq!(groups[0].object_type, "duplicate");
        assert_eq!(groups[0].total_wasted, 48);
        assert!(groups[0].sample_value.is_some());
        assert!(groups[0].sample_value.as_ref().unwrap().contains("duplicate"));
    }

    #[test]
    fn test_shallow_size_calculation() {
        let strings = vec![
            "".to_string(),
            "parent".to_string(),
            "child".to_string(),
            "prop".to_string(),
        ];
        let string_table = Arc::new(StringTable::new(strings));
        
        let mut graph = CompactGraph::new(string_table);
        
        // Add parent object (node 0) with size 100
        graph.node_types.push(3); // object
        graph.node_names.push(1); // "parent"
        graph.node_ids.push(1);
        graph.node_sizes.push(100);
        graph.node_edge_ranges.push((0, 1)); // 1 edge
        
        // Add child object (node 1) with size 50
        graph.node_types.push(3); // object
        graph.node_names.push(2); // "child"
        graph.node_ids.push(2);
        graph.node_sizes.push(50);
        graph.node_edge_ranges.push((1, 1)); // 0 edges
        
        // Add edge from parent to child
        graph.edge_types.push(2); // property
        graph.edge_names.push(3); // "prop"
        graph.edge_targets.push(1); // target node 1
        
        let analyzer = DuplicateAnalyzer::new(graph, false);
        
        // Calculate size of parent (should be shallow size only)
        let size = analyzer.calculate_total_size(0);
        assert_eq!(size, 100, "Should return shallow size of parent object");
    }
}
