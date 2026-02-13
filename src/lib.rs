mod types;
mod parser;
mod graph;
mod analysis;
mod paths;
mod report;

use analysis::duplicates::{DuplicateAnalyzer, DuplicateGroup};
use analysis::hidden_classes::{HiddenClassAnalyzer, HiddenClassGroup};
use anyhow::Result;
use graph::GraphBuilder;
use parser::SnapshotParser;
use paths::{RetentionPathFinder, RetentionPath};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use types::NodeId;

pub use analysis::duplicates::DuplicateGroup as PublicDuplicateGroup;
pub use analysis::hidden_classes::HiddenClassGroup as PublicHiddenClassGroup;
pub use paths::RetentionPath as PublicRetentionPath;

pub struct AnalysisResults {
    pub duplicate_groups: Vec<DuplicateGroup>,
    pub hidden_class_groups: Vec<HiddenClassGroup>,
    pub retention_paths: HashMap<NodeId, Vec<RetentionPath>>,
}

pub fn analyze_snapshot(input_path: &PathBuf, include_hidden_classes: bool) -> Result<AnalysisResults> {
    // Parse snapshot
    let parser = SnapshotParser::new(input_path)?;
    let (metadata, string_table) = parser.parse_metadata_and_strings()?;
    let string_table = Arc::new(string_table);

    // Build graph
    let (nodes, edges) = parser.parse_nodes_and_edges()?;
    let mut builder = GraphBuilder::new(metadata, string_table.clone());
    builder.add_nodes(&nodes)?;
    builder.add_edges(&edges)?;
    let graph = builder.finalize();

    // Analyze duplicates
    let analyzer = DuplicateAnalyzer::new(graph, include_hidden_classes);
    let duplicate_groups = analyzer.find_duplicates();
    let graph = analyzer.into_graph();

    // Analyze hidden classes
    let hc_analyzer = HiddenClassAnalyzer::new(graph);
    let hidden_class_groups = hc_analyzer.analyze();
    let graph = hc_analyzer.into_graph();

    // Find retention paths for top duplicates
    let path_finder = RetentionPathFinder::new(&graph);
    let mut retention_paths = HashMap::new();
    
    for group in duplicate_groups.iter().take(10) {
        let paths = path_finder.find_paths(group.representative, 3);
        if !paths.is_empty() {
            retention_paths.insert(group.representative, paths);
        }
    }

    Ok(AnalysisResults {
        duplicate_groups,
        hidden_class_groups,
        retention_paths,
    })
}
