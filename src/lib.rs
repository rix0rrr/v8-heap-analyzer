mod types;
mod parser;
mod graph;
mod analysis;
mod paths;
mod report;
mod utils;

use analysis::duplicates::{DuplicateAnalyzer, DuplicateGroup};
use analysis::hidden_classes::{HiddenClassAnalyzer, HiddenClassGroup};
use anyhow::Result;
use graph::{CompactGraph, GraphBuilder};
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

/// Builds a CompactGraph from a heap snapshot file
pub fn build_graph_from_snapshot(input_path: &PathBuf) -> Result<CompactGraph> {
    let parser = SnapshotParser::new(input_path)?;
    let (metadata, string_table) = parser.parse_metadata_and_strings()?;
    let string_table = Arc::new(string_table);

    let (nodes, edges) = parser.parse_nodes_and_edges()?;
    let mut builder = GraphBuilder::new(metadata, string_table);
    builder.add_nodes(&nodes)?;
    builder.add_edges(&edges)?;
    
    Ok(builder.finalize())
}

pub fn analyze_snapshot(input_path: &PathBuf, include_hidden_classes: bool) -> Result<AnalysisResults> {
    let graph = build_graph_from_snapshot(input_path)?;
    analyze_graph(graph, include_hidden_classes)
}

/// Analyzes a CompactGraph for duplicates, hidden classes, and retention paths
pub fn analyze_graph(graph: CompactGraph, include_hidden_classes: bool) -> Result<AnalysisResults> {
    // Analyze duplicates
    let analyzer = DuplicateAnalyzer::new(graph, include_hidden_classes);
    let duplicate_groups = analyzer.find_duplicates();
    let graph = analyzer.into_graph();

    // Analyze hidden classes
    let hc_analyzer = HiddenClassAnalyzer::new(graph);
    let hidden_class_groups = hc_analyzer.analyze();
    let graph = hc_analyzer.into_graph();

    // Find retention paths for top duplicates
    let retention_paths = find_retention_paths_for_groups(&graph, &duplicate_groups, 10, 3);

    Ok(AnalysisResults {
        duplicate_groups,
        hidden_class_groups,
        retention_paths,
    })
}

/// Finds retention paths for the top N duplicate groups
fn find_retention_paths_for_groups(
    graph: &CompactGraph,
    duplicate_groups: &[DuplicateGroup],
    top_n: usize,
    max_paths_per_group: usize,
) -> HashMap<NodeId, Vec<RetentionPath>> {
    let path_finder = RetentionPathFinder::new(graph);
    let mut retention_paths = HashMap::new();
    
    for group in duplicate_groups.iter().take(top_n) {
        let paths = path_finder.find_paths(group.representative, max_paths_per_group);
        if !paths.is_empty() {
            retention_paths.insert(group.representative, paths);
        }
    }
    
    retention_paths
}

