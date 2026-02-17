#![allow(dead_code)]
mod analysis;
mod graph;
mod parser;
mod paths;
mod report;
mod snapshot;
mod types;
mod utils;

use anyhow::Result;
use clap::Parser;
use std::collections::HashMap;
use std::io::Write;
use std::io::stdout;
use std::path::PathBuf;
use std::time::Instant;

// Import the shared analysis functions
use crate::graph::v8_heap_graph::V8HeapGraph;
use crate::snapshot::read_v8_snapshot_file;

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
    let snap = read_v8_snapshot_file(&cli.input)?;
    let graph = V8HeapGraph::from(snap);
    std::mem::drop(_t);

    println!("Nodes: {}", graph.node_count());
    println!("Edges: {}", graph.edge_count());

    let _t = start_timer("Calculating dominators (Lengauer Tarjan)".into());
    let lt = analysis::lengauer_tarjan::lengauer_tarjan(&graph, &[0]);
    std::mem::drop(_t);

    let _t = start_timer("Calculating dominators (Cooper's)".into());
    let out = petgraph::algo::dominators::simple_fast(&graph, 0);
    std::mem::drop(_t);

    let coop: HashMap<_, _> = graph
        .nodes()
        .flat_map(|i| out.immediate_dominator(i).map(|d| (i, d)))
        .collect();

    println!("Cooper's length: {}", coop.len());
    println!("LT length: {}", lt.len());
    for node in graph.nodes() {
        let c = coop.get(&node);
        let l = lt.get(&node);
        if c != l {
            println!("Node {} -> Cooper {:?}, LT {:?}", node, c, l);
        }
    }

    //    println!("{:?}", snap);

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
