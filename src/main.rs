mod analysis;
mod graph;
mod parser;
mod paths;
mod report;
mod snapshot;
mod types;
mod utils;

use anyhow::{Context, Result};
use clap::Parser;
use parser::SnapshotParser;
use report::generator::ReportGenerator;
use std::fs::File;
use std::io::{Write, stdout};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

// Import the shared analysis functions
use crate::analysis::duplicates::DuplicateAnalyzer;
use crate::analysis::hidden_classes::HiddenClassAnalyzer;
use crate::graph::GraphBuilder;
use crate::graph::v8_heap_graph::V8HeapGraph;
use crate::paths::RetentionPathFinder;
use crate::snapshot::read_v8_snapshot_file;
use std::collections::HashMap;

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
    println!();

    // Full serde
    let _t = start_timer(format!("Loading {}", cli.input.display()));
    stdout().flush()?;
    let snap = read_v8_snapshot_file(&cli.input)?;
    let graph = V8HeapGraph::from(snap);
    std::mem::drop(_t);

    println!("Nodes: {}", graph.node_count());
    println!("Edges: {}", graph.edge_count());

    let _t = start_timer("Calculating dominators".into());
    stdout().flush()?;
    let out = petgraph::algo::dominators::simple_fast(&graph, 0);
    std::mem::drop(_t);

    //    println!("{:?}", snap);

    Ok(())
}

fn main2() -> Result<()> {
    let cli = Cli::parse();

    println!("V8 Heap Analyzer v0.1.0");
    println!("Analyzing: {}", cli.input.display());
    println!();

    // Parse snapshot metadata for progress reporting
    println!("Parsing snapshot...");
    let parser = SnapshotParser::new(&cli.input)?;
    let (metadata, string_table) = parser.parse_metadata_and_strings()?;
    let string_table = Arc::new(string_table);

    let (node_count, edge_count) = parser.get_actual_counts(&metadata)?;
    println!("  Nodes: {}", node_count);
    println!("  Edges: {}", edge_count);
    println!("  Strings: {}", string_table.len());
    println!();

    // Build graph
    println!("Building graph...");
    let (nodes, edges) = parser.parse_nodes_and_edges()?;
    let mut builder = GraphBuilder::new(metadata, string_table);
    builder.add_nodes(&nodes)?;
    builder.add_edges(&edges)?;
    let graph = builder.finalize();
    println!();

    // Calculate retained sizes
    println!("Calculating retained sizes...");
    let retained_sizes = analysis::retained_size::calculate_retained_sizes(&graph);
    println!("  Calculated sizes for {} nodes", retained_sizes.len());
    println!();

    // Analyze duplicates
    println!("Analyzing duplicates...");
    let analyzer = DuplicateAnalyzer::new(graph, cli.include_hidden_classes);
    let mut duplicate_groups = analyzer.find_duplicates();
    let graph = analyzer.into_graph();

    // Enrich with retained sizes
    DuplicateAnalyzer::enrich_with_retained_sizes(&mut duplicate_groups, &retained_sizes);

    println!("  Found {} duplicate groups", duplicate_groups.len());
    println!();

    // Analyze hidden classes
    println!("Analyzing hidden classes...");
    let hc_analyzer = HiddenClassAnalyzer::new(graph);
    let hidden_class_groups = hc_analyzer.analyze();
    let graph = hc_analyzer.into_graph();
    println!(
        "  Found {} object types with hidden classes",
        hidden_class_groups.len()
    );
    println!();

    // Find retention paths
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
    generate_report(
        &cli,
        &graph,
        duplicate_groups,
        hidden_class_groups,
        retention_paths,
    )?;

    println!("Done!");
    Ok(())
}

fn generate_report(
    cli: &Cli,
    graph: &graph::CompactGraph,
    duplicate_groups: Vec<analysis::duplicates::DuplicateGroup>,
    hidden_class_groups: Vec<analysis::hidden_classes::HiddenClassGroup>,
    retention_paths: HashMap<types::NodeId, Vec<paths::RetentionPath>>,
) -> Result<()> {
    println!("Generating report...");
    let generator = ReportGenerator::new(
        graph,
        duplicate_groups,
        hidden_class_groups,
        retention_paths,
    );

    if let Some(output_path) = &cli.output {
        let mut output_file = File::create(output_path).context("Failed to create output file")?;

        match cli.format.as_str() {
            "json" => generator.generate_json_report(&mut output_file, 50)?,
            _ => generator.generate_text_report(&mut output_file, 10)?,
        }

        println!("Report written to: {}", output_path.display());
    } else {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();

        match cli.format.as_str() {
            "json" => generator.generate_json_report(&mut handle, 50)?,
            _ => generator.generate_text_report(&mut handle, 10)?,
        }
    }

    Ok(())
}

struct Timer {
    name: String,
    start: Instant,
}

fn start_timer(name: String) -> Timer {
    print!("{}... ", name);
    let _ = stdout().flush();
    Timer {
        name,
        start: Instant::now(),
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let duration = Instant::now() - self.start;
        println!("Done ({:?})", duration);
    }
}
