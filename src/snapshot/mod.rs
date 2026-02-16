use anyhow::{Context, Result};
use std::{fs::File, io::BufReader, path::Path};

use serde::{Deserialize, de::Visitor};

use crate::types::NodeId;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SnapshotType {
    Str(String),
    Strs(Vec<String>),
}

#[derive(Debug, Deserialize)]
pub struct SnapshotMetadata {
    pub edge_fields: Vec<String>,
    pub edge_types: Vec<SnapshotType>, // string or string[]
    pub location_fields: Vec<String>,
    pub node_fields: Vec<String>,
    pub node_types: Vec<SnapshotType>, // string or string[]
    pub sample_fields: Vec<String>,
    pub trace_function_info_fields: Vec<String>,
    pub trace_node_fields: Vec<String>,
}

impl SnapshotMetadata {
    pub fn node_field_count(&self) -> usize {
        self.node_fields.len()
    }

    pub fn edge_field_count(&self) -> usize {
        self.edge_fields.len()
    }
}

#[derive(Debug, Deserialize)]
pub struct Snapshot {
    pub meta: SnapshotMetadata,
    pub node_count: usize,
    pub edge_count: usize,
    pub trace_function_count: usize,
    pub extra_native_bytes: usize,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotFile {
    pub snapshot: Snapshot,
    pub nodes: Vec<NodeId>,
    pub edges: Vec<NodeId>,
    pub locations: Vec<NodeId>,
    pub samples: Vec<serde_json::Value>, // ?
    pub strings: Vec<String>,
    pub trace_function_infos: Vec<serde_json::Value>, // ?
    pub trace_tree: Vec<serde_json::Value>,           // ?
}

pub fn read_v8_snapshot_file(path: &Path) -> Result<SnapshotFile> {
    let file = File::open(path).context("Failed to open snapshot file")?;
    let reader = BufReader::new(file);

    let snapshot: SnapshotFile =
        serde_json::from_reader(reader).context("Failed to parse snapshot JSON")?;

    Ok(snapshot)
}
