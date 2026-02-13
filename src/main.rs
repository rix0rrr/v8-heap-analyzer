mod types;
mod parser;
mod graph;
mod analysis;
mod paths;
mod report;

use analysis::duplicates::DuplicateAnalyzer;
use analysis::hidden_classes::HiddenClassAnalyzer;
use anyhow::{Context, Result};
use clap::Parser;
use graph::GraphBuilder;
use parser::SnapshotParser;
use paths::RetentionPathFinder;
use report::ReportGenerator;
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "v8-heap-analyzer")]
#[command(about = "Analyze V8 heap snapshots for duplicates and memory issues")]
struct Cli {
    /// Input heap snapshot file
    #[arg(short, long)]
    input: PathBuf,

    /// Output report file (defaults to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format (text or json)
    #[arg(short, long, default_value = "text")]
    format: String,

    /// Include hidden classes in duplicate detection
    #[arg(long, default_value = "false")]
    include_hidden_classes: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    println!("V8 Heap Analyzer v0.1.0");
    println!("Analyzing: {}", cli.input.display());
    println!();

    // Parse snapshot
    println!("Parsing snapshot...");
    let parser = SnapshotParser::new(&cli.input)?;
    let (metadata, string_table) = parser.parse_metadata_and_strings()?;
    let string_table = Arc::new(string_table);
    
    // Get actual counts from the data
    let (node_count, edge_count) = parser.get_actual_counts(&metadata)?;
    
    println!("  Nodes: {}", node_count);
    println!("  Edges: {}", edge_count);
    println!("  Strings: {}", string_table.len());
    println!();

    // Build graph
    println!("Building graph...");
    let (nodes, edges) = parser.parse_nodes_and_edges()?;
    let mut builder = GraphBuilder::new(metadata, string_table.clone());
    builder.add_nodes(&nodes)?;
    builder.add_edges(&edges)?;
    let graph = builder.finalize();
    println!();

    // Analyze duplicates
    println!("Analyzing duplicates...");
    let analyzer = DuplicateAnalyzer::new(graph, cli.include_hidden_classes);
    let duplicate_groups = analyzer.find_duplicates();
    let graph = analyzer.into_graph();
    println!("  Found {} duplicate groups", duplicate_groups.len());
    println!();

    // Analyze hidden classes
    println!("Analyzing hidden classes...");
    let hc_analyzer = HiddenClassAnalyzer::new(graph);
    let hidden_class_groups = hc_analyzer.analyze();
    let graph = hc_analyzer.into_graph();
    println!("  Found {} object types with hidden classes", hidden_class_groups.len());
    println!();

    // Find retention paths for top duplicates
    println!("Finding retention paths...");
    let path_finder = RetentionPathFinder::new(&graph);
    let mut retention_paths = HashMap::new();
    
    for group in duplicate_groups.iter().take(10) {
        let paths = path_finder.find_paths(group.representative, 3);
        if !paths.is_empty() {
            retention_paths.insert(group.representative, paths);
        }
    }
    println!("  Found paths for {} groups", retention_paths.len());
    println!();

    // Generate report
    println!("Generating report...");
    let generator = ReportGenerator::new(&graph, duplicate_groups, hidden_class_groups, retention_paths);
    
    if let Some(output_path) = &cli.output {
        let mut output_file = File::create(output_path)
            .context("Failed to create output file")?;
        
        match cli.format.as_str() {
            "json" => generator.generate_json_report(&mut output_file, 50)?,
            _ => generator.generate_text_report(&mut output_file, 10)?,
        }
        
        println!("Report written to: {}", output_path.display());
    } else {
        // Write to stdout
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        
        match cli.format.as_str() {
            "json" => generator.generate_json_report(&mut handle, 50)?,
            _ => generator.generate_text_report(&mut handle, 10)?,
        }
    }
    
    println!("Done!");

    Ok(())
}
