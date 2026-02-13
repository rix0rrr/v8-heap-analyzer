mod types;
mod parser;
mod graph;
mod analysis;
mod paths;
mod report;
mod utils;

use analysis::duplicates::{DuplicateAnalyzer, DuplicateGroup};
use analysis::hidden_classes::{HiddenClassAnalyzer, HiddenClassGroup};
use analysis::retained_size::calculate_retained_sizes;
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
    // Note: Retained size calculation is disabled by default as it's O(n²) and very slow for large graphs
    // Uncomment to enable:
    // println!("Calculating retained sizes...");
    // let retained_sizes = calculate_retained_sizes(&graph);
    // println!("  Calculated sizes for {} nodes", retained_sizes.len());
    // println!();
    
    // Analyze duplicates
    let analyzer = DuplicateAnalyzer::new(graph, include_hidden_classes);
    let mut duplicate_groups = analyzer.find_duplicates();
    let graph = analyzer.into_graph();
    
    // Enrich duplicate groups with retained sizes (if calculated)
    // DuplicateAnalyzer::enrich_with_retained_sizes(&mut duplicate_groups, &retained_sizes);

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
        let mut paths = path_finder.find_paths(group.representative, max_paths_per_group);
        if !paths.is_empty() {
            filter_subset_paths(&mut paths);
            retention_paths.insert(group.representative, paths);
        }
    }
    
    retention_paths
}

/// Removes paths that are entirely contained within other paths
fn filter_subset_paths(paths: &mut Vec<RetentionPath>) {
    let mut to_remove = Vec::new();
    
    for i in 0..paths.len() {
        for j in 0..paths.len() {
            if i != j && is_subset(&paths[i].nodes, &paths[j].nodes) {
                to_remove.push(i);
                break;
            }
        }
    }
    
    // Remove in reverse order to maintain indices
    to_remove.sort_unstable();
    to_remove.dedup();
    for &idx in to_remove.iter().rev() {
        paths.remove(idx);
    }
}

/// Check if path_a is a subset of path_b (all nodes of a appear in order in b)
fn is_subset(path_a: &[NodeId], path_b: &[NodeId]) -> bool {
    if path_a.len() >= path_b.len() {
        return false;
    }
    
    // Check if all nodes in path_a appear consecutively in path_b
    path_b.windows(path_a.len()).any(|window| window == path_a)
}

