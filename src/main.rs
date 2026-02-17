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
use itertools::Itertools;
use petgraph::visit::Bfs;
use std::path::PathBuf;

use crate::analysis::dominator_tree::DominatorTree;
use crate::analysis::dominator_tree::tree_from_immediate_dominators;
use crate::graph::gexf::write_gexf_file;
use crate::graph::gml::write_gml_file;
use crate::graph::v8_heap_graph::EdgeType;
use crate::graph::v8_heap_graph::NodeType;
// Import the shared analysis functions
use crate::graph::v8_heap_graph::V8HeapGraph;
use crate::snapshot::read_v8_snapshot_file;
use crate::types::NodeId;
use crate::utils::format_bytes;
use crate::utils::print_safe;
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

    println!("Nodes:       {}", graph.node_count());
    println!("Edges:       {}", graph.edge_count());
    println!("Memory used: {}", format_bytes(graph.mem_size()));

    let root: NodeId = 0;
    let _t = start_timer("Calculating dominators (Lengauer Tarjan)".into());
    let lt = graph::lengauer_tarjan::lengauer_tarjan(&graph, &[root]);
    std::mem::drop(_t);

    let tree = tree_from_immediate_dominators(lt, &graph);
    // print_dominator_tree(&tree, &graph);
    print_graph(&graph, &tree);
    // write_gml_file(Path::new("graph.gml"), &graph)?;

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

fn print_graph(graph: &V8HeapGraph, dom_tree: &DominatorTree) {
    let mut bfs = Bfs::new(&graph, 0);
    while let Some(nx) = bfs.next(&graph) {
        let node = graph.node(nx);

        println!(
            "node {} type={} name={} stable_id={} self_size={} retained_size={}",
            nx,
            node.typ_str(),
            node.print_safe_name(40),
            node.stable_id(),
            node.self_size(),
            dom_tree.retained_size(nx)
        );

        for edge in graph.edges_for(nx) {
            println!(
                "    --[{}:{}]--> {}  {}",
                edge.typ_str(),
                edge.name_or_index(),
                edge.to_node(),
                minimal_node_repr(edge.to_node(), graph),
            );
        }
        println!("");
    }
}

fn minimal_node_repr(node: NodeId, graph: &V8HeapGraph) -> String {
    let node = graph.node(node);
    if node.typ() == NodeType::String {
        return print_safe(node.name(), 30);
    }

    if node.typ() == NodeType::Object {
        if graph
            .find_edge(node.id, EdgeType::Internal, "elements")
            .is_some()
        {
            // It's an array or array-like, format like an array
            return format!(
                "[ {} ]",
                graph
                    .edges_for(node.id)
                    .filter(|e| e.typ() == EdgeType::Element)
                    .map(|e| minimal_node_repr(e.to_node(), graph))
                    .join(", ")
            );
        }

        // Format as a regular object
        return format!(
            "{{ {} }}",
            graph
                .edges_for(node.id)
                .filter(|e| e.typ() == EdgeType::Property)
                .map(|e| e.name_or_index().to_string())
                .join(", ")
        );
    }

    format!("{}:{}", node.typ_str(), node.print_safe_name(30))
}
