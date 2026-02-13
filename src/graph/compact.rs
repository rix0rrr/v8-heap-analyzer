use crate::parser::StringTable;
use crate::types::NodeId;
use std::sync::Arc;

pub struct CompactGraph {
    // Node data (Structure of Arrays)
    pub(crate) node_types: Vec<u8>,
    pub(crate) node_names: Vec<u32>,
    pub(crate) node_ids: Vec<u32>,
    pub(crate) node_sizes: Vec<u32>,
    pub(crate) node_edge_ranges: Vec<(u32, u32)>,
    
    // Edge data
    pub(crate) edge_types: Vec<u8>,
    pub(crate) edge_names: Vec<u32>,
    pub(crate) edge_targets: Vec<u32>,
    
    // Metadata
    pub(crate) string_table: Arc<StringTable>,
    pub(crate) gc_roots: Vec<NodeId>,
}

impl CompactGraph {
    pub fn new(string_table: Arc<StringTable>) -> Self {
        Self {
            node_types: Vec::new(),
            node_names: Vec::new(),
            node_ids: Vec::new(),
            node_sizes: Vec::new(),
            node_edge_ranges: Vec::new(),
            edge_types: Vec::new(),
            edge_names: Vec::new(),
            edge_targets: Vec::new(),
            string_table,
            gc_roots: Vec::new(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.node_types.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edge_types.len()
    }

    pub fn node_type(&self, node_id: NodeId) -> Option<u8> {
        self.node_types.get(node_id as usize).copied()
    }

    pub fn node_name(&self, node_id: NodeId) -> Option<&str> {
        let name_idx = *self.node_names.get(node_id as usize)?;
        self.string_table.get(name_idx)
    }

    pub fn node_size(&self, node_id: NodeId) -> Option<u32> {
        self.node_sizes.get(node_id as usize).copied()
    }

    pub fn edges(&self, node_id: NodeId) -> EdgeIterator {
        let (start, end) = self.node_edge_ranges
            .get(node_id as usize)
            .copied()
            .unwrap_or((0, 0));
        
        EdgeIterator {
            graph: self,
            current: start,
            end,
        }
    }

    pub fn is_gc_root(&self, node_id: NodeId) -> bool {
        self.gc_roots.contains(&node_id)
    }

    pub fn gc_roots(&self) -> &[NodeId] {
        &self.gc_roots
    }
}

pub struct EdgeIterator<'a> {
    graph: &'a CompactGraph,
    current: u32,
    end: u32,
}

impl<'a> Iterator for EdgeIterator<'a> {
    type Item = Edge<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.end {
            return None;
        }

        let idx = self.current as usize;
        let edge = Edge {
            edge_type: self.graph.edge_types[idx],
            name_or_index: self.graph.edge_names[idx],
            target: self.graph.edge_targets[idx],
            string_table: &self.graph.string_table,
        };

        self.current += 1;
        Some(edge)
    }
}

pub struct Edge<'a> {
    pub edge_type: u8,
    pub name_or_index: u32,
    pub target: NodeId,
    string_table: &'a StringTable,
}

impl<'a> Edge<'a> {
    pub fn name(&self) -> Option<&str> {
        self.string_table.get(self.name_or_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_graph() -> CompactGraph {
        let strings = vec!["".to_string(), "Window".to_string(), "document".to_string()];
        let string_table = Arc::new(StringTable::new(strings));
        
        let mut graph = CompactGraph::new(string_table);
        
        // Add two nodes
        graph.node_types.push(3); // object type
        graph.node_names.push(1); // "Window"
        graph.node_ids.push(1);
        graph.node_sizes.push(100);
        graph.node_edge_ranges.push((0, 1)); // 1 edge
        
        graph.node_types.push(3); // object type
        graph.node_names.push(2); // "document"
        graph.node_ids.push(2);
        graph.node_sizes.push(200);
        graph.node_edge_ranges.push((1, 1)); // 0 edges
        
        // Add one edge from node 0 to node 1
        graph.edge_types.push(2); // property
        graph.edge_names.push(2); // "document"
        graph.edge_targets.push(1); // target node 1
        
        // Mark node 0 as GC root
        graph.gc_roots.push(0);
        
        graph
    }

    #[test]
    fn test_graph_counts() {
        let graph = create_test_graph();
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_node_accessors() {
        let graph = create_test_graph();
        
        assert_eq!(graph.node_type(0), Some(3));
        assert_eq!(graph.node_name(0), Some("Window"));
        assert_eq!(graph.node_size(0), Some(100));
        
        assert_eq!(graph.node_type(1), Some(3));
        assert_eq!(graph.node_name(1), Some("document"));
        assert_eq!(graph.node_size(1), Some(200));
        
        assert_eq!(graph.node_type(2), None);
    }

    #[test]
    fn test_edge_iteration() {
        let graph = create_test_graph();
        
        let edges: Vec<_> = graph.edges(0).collect();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].edge_type, 2);
        assert_eq!(edges[0].name(), Some("document"));
        assert_eq!(edges[0].target, 1);
        
        let edges: Vec<_> = graph.edges(1).collect();
        assert_eq!(edges.len(), 0);
    }

    #[test]
    fn test_gc_roots() {
        let graph = create_test_graph();
        assert!(graph.is_gc_root(0));
        assert!(!graph.is_gc_root(1));
        assert_eq!(graph.gc_roots(), &[0]);
    }
}
