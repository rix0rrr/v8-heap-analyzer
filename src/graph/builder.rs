use crate::graph::CompactGraph;
use crate::parser::{decode_edge, decode_node, SnapshotMetadata, StringTable};
use crate::types::NodeId;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;

pub struct GraphBuilder {
    metadata: SnapshotMetadata,
    string_table: Arc<StringTable>,
    
    // Node data
    node_types: Vec<u8>,
    node_names: Vec<u32>,
    node_ids: Vec<u32>,
    node_sizes: Vec<u32>,
    node_edge_counts: Vec<u32>,
    
    // Edge data
    edge_types: Vec<u8>,
    edge_names: Vec<u32>,
    edge_targets: Vec<u32>,
    
    gc_roots: Vec<NodeId>,
}

impl GraphBuilder {
    pub fn new(metadata: SnapshotMetadata, string_table: Arc<StringTable>) -> Self {
        Self {
            metadata,
            string_table,
            node_types: Vec::new(),
            node_names: Vec::new(),
            node_ids: Vec::new(),
            node_sizes: Vec::new(),
            node_edge_counts: Vec::new(),
            edge_types: Vec::new(),
            edge_names: Vec::new(),
            edge_targets: Vec::new(),
            gc_roots: Vec::new(),
        }
    }

    pub fn add_nodes(&mut self, nodes: &[u64]) -> Result<()> {
        let field_count = self.metadata.node_field_count();
        let pb = ProgressBar::new((nodes.len() / field_count) as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} nodes")
                .unwrap()
                .progress_chars("##-"),
        );

        for chunk in nodes.chunks(field_count) {
            if let Some(node) = decode_node(chunk, &self.metadata) {
                let node_idx = self.node_types.len() as NodeId;
                
                self.node_types.push(node.node_type);
                self.node_names.push(node.name_idx);
                self.node_ids.push(node.id);
                self.node_sizes.push(node.size);
                self.node_edge_counts.push(node.edge_count);
                
                // Check if this is a GC root (type 0 = synthetic, often GC roots)
                if node.node_type == 0 || node.node_type == 9 {
                    self.gc_roots.push(node_idx);
                }
                
                pb.inc(1);
            }
        }
        
        pb.finish_with_message("Nodes loaded");
        Ok(())
    }

    pub fn add_edges(&mut self, edges: &[u64]) -> Result<()> {
        let field_count = self.metadata.edge_field_count();
        let pb = ProgressBar::new((edges.len() / field_count) as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} edges")
                .unwrap()
                .progress_chars("##-"),
        );

        for chunk in edges.chunks(field_count) {
            if let Some(edge) = decode_edge(chunk, &self.metadata) {
                self.edge_types.push(edge.edge_type);
                self.edge_names.push(edge.name_or_index);
                // Convert target from flat array index to node index
                let target_node_idx = edge.target_node / self.metadata.node_field_count() as u32;
                self.edge_targets.push(target_node_idx);
                
                pb.inc(1);
            }
        }
        
        pb.finish_with_message("Edges loaded");
        Ok(())
    }

    pub fn finalize(self) -> CompactGraph {
        // Calculate edge ranges for each node
        let mut node_edge_ranges = Vec::with_capacity(self.node_types.len());
        let mut edge_start = 0u32;
        
        for &edge_count in &self.node_edge_counts {
            let edge_end = edge_start + edge_count;
            node_edge_ranges.push((edge_start, edge_end));
            edge_start = edge_end;
        }

        CompactGraph {
            node_types: self.node_types,
            node_names: self.node_names,
            node_ids: self.node_ids,
            node_sizes: self.node_sizes,
            node_edge_ranges,
            edge_types: self.edge_types,
            edge_names: self.edge_names,
            edge_targets: self.edge_targets,
            string_table: self.string_table,
            gc_roots: self.gc_roots,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_builder() {
        let strings = vec!["".to_string(), "Window".to_string(), "document".to_string()];
        let string_table = Arc::new(StringTable::new(strings));
        
        let meta_json = r#"{
            "node_fields": ["type", "name", "id", "self_size", "edge_count"],
            "node_types": [["object"], "string", "number", "number", "number"],
            "edge_fields": ["type", "name_or_index", "to_node"],
            "edge_types": [["property"], "string_or_number", "node"]
        }"#;
        let metadata: SnapshotMetadata = serde_json::from_str(meta_json).unwrap();
        
        let mut builder = GraphBuilder::new(metadata, string_table);
        
        // Add two nodes
        let nodes = vec![
            3, 1, 100, 48, 1,  // Node 0: type=3, name=1, id=100, size=48, edges=1
            3, 2, 200, 96, 0,  // Node 1: type=3, name=2, id=200, size=96, edges=0
        ];
        builder.add_nodes(&nodes).unwrap();
        
        // Add one edge (from node 0 to node 1)
        let edges = vec![
            2, 2, 5,  // Edge: type=2, name=2, target=5 (which is node 1 in flat array)
        ];
        builder.add_edges(&edges).unwrap();
        
        let graph = builder.finalize();
        
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.node_name(0), Some("Window"));
        assert_eq!(graph.node_name(1), Some("document"));
        
        let edges: Vec<_> = graph.edges(0).collect();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].target, 1);
    }
}
