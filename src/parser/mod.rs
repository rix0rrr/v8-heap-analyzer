pub mod metadata;
pub mod string_table;

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Deserializer;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek};
use std::path::Path;

pub use metadata::SnapshotMetadata;
pub use string_table::StringTable;

#[derive(Deserialize)]
struct SnapshotFile {
    snapshot: SnapshotSection,
    strings: Vec<String>,
    #[serde(default)]
    nodes: Vec<u64>,
    #[serde(default)]
    edges: Vec<u64>,
}

#[derive(Deserialize)]
struct SnapshotSection {
    meta: SnapshotMetadata,
}

pub struct SnapshotParser {
    path: std::path::PathBuf,
}

impl SnapshotParser {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            anyhow::bail!("Snapshot file does not exist: {}", path.display());
        }
        Ok(Self { path })
    }

    pub fn parse_metadata_and_strings(&self) -> Result<(SnapshotMetadata, StringTable)> {
        let file = File::open(&self.path).context("Failed to open snapshot file")?;
        let reader = BufReader::new(file);
        
        let snapshot: SnapshotFile = serde_json::from_reader(reader)
            .context("Failed to parse snapshot JSON")?;

        Ok((snapshot.snapshot.meta, StringTable::new(snapshot.strings)))
    }

    pub fn parse_nodes_and_edges(&self) -> Result<(Vec<u64>, Vec<u64>)> {
        let file = File::open(&self.path).context("Failed to open snapshot file")?;
        let reader = BufReader::new(file);
        
        let snapshot: SnapshotFile = serde_json::from_reader(reader)
            .context("Failed to parse snapshot JSON")?;

        Ok((snapshot.nodes, snapshot.edges))
    }

    pub fn get_actual_counts(&self, metadata: &SnapshotMetadata) -> Result<(usize, usize)> {
        let (nodes, edges) = self.parse_nodes_and_edges()?;
        let node_count = nodes.len() / metadata.node_field_count();
        let edge_count = edges.len() / metadata.edge_field_count();
        Ok((node_count, edge_count))
    }
}

pub struct DecodedNode {
    pub node_type: u8,
    pub name_idx: u32,
    pub id: u32,
    pub size: u32,
    pub edge_count: u32,
}

pub struct DecodedEdge {
    pub edge_type: u8,
    pub name_or_index: u32,
    pub target_node: u32,
}

pub fn decode_node(data: &[u64], meta: &SnapshotMetadata) -> Option<DecodedNode> {
    let field_count = meta.node_field_count();
    if data.len() < field_count {
        return None;
    }

    Some(DecodedNode {
        node_type: data[0] as u8,
        name_idx: data[1] as u32,
        id: data[2] as u32,
        size: data[3] as u32,
        edge_count: data[4] as u32,
    })
}

pub fn decode_edge(data: &[u64], meta: &SnapshotMetadata) -> Option<DecodedEdge> {
    let field_count = meta.edge_field_count();
    if data.len() < field_count {
        return None;
    }

    Some(DecodedEdge {
        edge_type: data[0] as u8,
        name_or_index: data[1] as u32,
        target_node: data[2] as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_minimal_snapshot() {
        let json = r#"{
            "snapshot": {
                "meta": {
                    "node_fields": ["type", "name", "id", "self_size", "edge_count"],
                    "node_types": [["object", "string"], "string", "number", "number", "number"],
                    "edge_fields": ["type", "name_or_index", "to_node"],
                    "edge_types": [["property", "element"], "string_or_number", "node"]
                }
            },
            "nodes": [3, 1, 100, 48, 1],
            "edges": [2, 2, 0],
            "strings": ["", "test", "example"]
        }"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let parser = SnapshotParser::new(temp_file.path()).unwrap();
        let (meta, strings) = parser.parse_metadata_and_strings().unwrap();

        assert_eq!(meta.node_fields.len(), 5);
        assert_eq!(meta.edge_fields.len(), 3);
        assert_eq!(strings.len(), 3);
        assert_eq!(strings.get(1), Some("test"));

        let (nodes, edges) = parser.parse_nodes_and_edges().unwrap();
        assert_eq!(nodes.len(), 5);
        assert_eq!(edges.len(), 3);
    }

    #[test]
    fn test_decode_node() {
        let json = r#"{
            "node_fields": ["type", "name", "id", "self_size", "edge_count"],
            "node_types": [["object"], "string", "number", "number", "number"],
            "edge_fields": ["type", "name_or_index", "to_node"],
            "edge_types": [["property"], "string_or_number", "node"]
        }"#;

        let meta: SnapshotMetadata = serde_json::from_str(json).unwrap();
        let node_data = vec![3, 1, 100, 48, 2];
        
        let decoded = decode_node(&node_data, &meta).unwrap();
        assert_eq!(decoded.node_type, 3);
        assert_eq!(decoded.name_idx, 1);
        assert_eq!(decoded.id, 100);
        assert_eq!(decoded.size, 48);
        assert_eq!(decoded.edge_count, 2);
    }

    #[test]
    fn test_decode_edge() {
        let json = r#"{
            "node_fields": ["type", "name", "id", "self_size", "edge_count"],
            "node_types": [["object"], "string", "number", "number", "number"],
            "edge_fields": ["type", "name_or_index", "to_node"],
            "edge_types": [["property"], "string_or_number", "node"]
        }"#;

        let meta: SnapshotMetadata = serde_json::from_str(json).unwrap();
        let edge_data = vec![2, 5, 10];
        
        let decoded = decode_edge(&edge_data, &meta).unwrap();
        assert_eq!(decoded.edge_type, 2);
        assert_eq!(decoded.name_or_index, 5);
        assert_eq!(decoded.target_node, 10);
    }
}
