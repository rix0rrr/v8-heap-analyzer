use crate::graph::CompactGraph;
use crate::types::NodeId;
use ahash::AHashMap;
use serde::Serialize;

pub struct HiddenClassAnalyzer {
    graph: CompactGraph,
}

#[derive(Debug, Clone, Serialize)]
pub struct HiddenClassGroup {
    pub object_type: String,
    pub hidden_class_count: usize,
    pub total_hidden_class_memory: u64,
    pub hidden_classes: Vec<NodeId>,
}

impl HiddenClassAnalyzer {
    pub fn new(graph: CompactGraph) -> Self {
        Self { graph }
    }

    pub fn analyze(&self) -> Vec<HiddenClassGroup> {
        // Map from object type to hidden classes
        let mut type_to_hidden_classes: AHashMap<String, Vec<NodeId>> = AHashMap::new();
        
        // Find all hidden class nodes (type 4 in V8)
        for node_id in 0..self.graph.node_count() as NodeId {
            if let Some(node_type) = self.graph.node_type(node_id) {
                if node_type == 4 {
                    // This is a hidden class, find what objects use it
                    if let Some(type_name) = self.find_object_type_for_hidden_class(node_id) {
                        type_to_hidden_classes
                            .entry(type_name)
                            .or_default()
                            .push(node_id);
                    }
                }
            }
        }
        
        // Create groups
        let mut groups: Vec<_> = type_to_hidden_classes
            .into_iter()
            .map(|(object_type, hidden_classes)| {
                let total_memory: u64 = hidden_classes
                    .iter()
                    .filter_map(|&id| self.graph.node_size(id))
                    .map(|s| s as u64)
                    .sum();
                
                HiddenClassGroup {
                    object_type,
                    hidden_class_count: hidden_classes.len(),
                    total_hidden_class_memory: total_memory,
                    hidden_classes,
                }
            })
            .collect();
        
        // Sort by total memory
        groups.sort_by(|a, b| b.total_hidden_class_memory.cmp(&a.total_hidden_class_memory));
        
        groups
    }

    fn find_object_type_for_hidden_class(&self, _hidden_class_id: NodeId) -> Option<String> {
        // Simplified: use the hidden class name as the type
        // In reality, we'd need to trace which objects reference this hidden class
        self.graph.node_name(_hidden_class_id).map(|s| s.to_string())
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
    fn test_hidden_class_analysis() {
        let strings = vec![
            "".to_string(),
            "User".to_string(),
            "Product".to_string(),
        ];
        let string_table = Arc::new(StringTable::new(strings));
        
        let mut graph = CompactGraph::new(string_table);
        
        // Add hidden class nodes
        graph.node_types.extend(&[4, 4, 4]); // Hidden class type
        graph.node_names.extend(&[1, 1, 2]); // Two for User, one for Product
        graph.node_ids.extend(&[1, 2, 3]);
        graph.node_sizes.extend(&[100, 100, 50]);
        graph.node_edge_ranges.extend(&[(0, 0), (0, 0), (0, 0)]);
        
        let analyzer = HiddenClassAnalyzer::new(graph);
        let groups = analyzer.analyze();
        
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].object_type, "User");
        assert_eq!(groups[0].hidden_class_count, 2);
        assert_eq!(groups[0].total_hidden_class_memory, 200);
    }
}
