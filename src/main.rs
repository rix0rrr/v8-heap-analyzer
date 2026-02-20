#![allow(dead_code)]
mod analysis;
mod graph;
mod paths;
mod report;
mod snapshot;
mod types;
mod utils;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use crate::analysis::all_paths::find_root_paths;
use crate::analysis::dominator_tree::tree_from_immediate_dominators;
// Import the shared analysis functions
use crate::graph::v8_heap_graph::V8HeapGraph;
use crate::report::{explore_graph, print_dominator_tree, print_graph};
use crate::snapshot::read_v8_snapshot_file;
use crate::types::NodeId;
use crate::utils::format_bytes;
use crate::utils::start_timer;

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

    /// Print the graph
    #[arg(short, long, default_value = "false")]
    print: bool,

    /// Print the dominator tree
    #[arg(short, long, default_value = "false")]
    tree: bool,

    /// Explore the dominator tree interactively
    #[arg(short, long, default_value = "false")]
    explore: bool,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    println!("V8 Heap Analyzer v0.1.0");
    println!();

    // Full serde
    let _t = start_timer(format!("Loading {}", args.input.display()));
    let snap = read_v8_snapshot_file(&args.input)?;
    let graph = V8HeapGraph::from(snap);
    std::mem::drop(_t);

    println!("Nodes:       {}", graph.total_node_count());
    println!("Edges:       {}", graph.total_edge_count());
    println!("Memory used: {}", format_bytes(graph.mem_size()));

    let root: NodeId = 0;
    let _t = start_timer("Calculating dominators".into());
    let lt = graph::lengauer_tarjan::lengauer_tarjan(&graph, &[root]);
    std::mem::drop(_t);

    let _t = start_timer("Converting dominators to tree".into());
    let tree = tree_from_immediate_dominators(lt, &graph);
    std::mem::drop(_t);

    let _t = start_timer("Finding root paths".into());
    let root_paths = find_root_paths(&graph, root);
    std::mem::drop(_t);

    if args.print {
        println!("");
        print_graph(&graph, &root_paths, &tree);
    }

    if args.tree {
        println!("");
        print_dominator_tree(&tree, &graph);
    }

    if args.explore {
        explore_graph(&tree, &root_paths, &graph)?;
    }

    /*
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
    */

    //    println!("{:?}", snap);

    Ok(())
}
