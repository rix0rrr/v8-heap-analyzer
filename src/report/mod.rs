use std::fmt::Write;

use itertools::Itertools;
use petgraph::visit::Bfs;

use crate::{
    analysis::{all_paths::RootPaths, dominator_tree::DominatorTree},
    graph::v8_heap_graph::{Edge, EdgeType, Node, NodeType, V8HeapGraph},
    types::NodeId,
    utils::{format_bytes, print_safe},
};

pub mod explorer;

pub use explorer::explore_graph;

pub fn print_graph(graph: &V8HeapGraph, root_paths: &RootPaths, dom_tree: &DominatorTree) {
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

        println!("    {}", minimal_node_repr(node.id, graph));

        let mut s = String::new();
        let _ = format_retention_paths(&mut s, node.id, root_paths, graph);
        print!("{}", s);

        for edge in graph.out_edges(nx) {
            println!(
                "    --[{}:{}]--> {}  {}",
                edge.typ_str(),
                edge.name_or_index(),
                edge.to_node(),
                minimal_node_repr(edge.to_node(), graph),
            );
        }
        println!();
    }
}

pub fn minimal_node_repr(node: NodeId, graph: &V8HeapGraph) -> String {
    let node = graph.node(node);

    match node.typ() {
        NodeType::String => print_safe(node.name(), 30),
        NodeType::Synthetic => node.name().to_string(),
        NodeType::ConcatString => {
            let first = graph
                .find_edge(node.id, EdgeType::Internal, "first")
                .expect("ConcatString must have first");
            let second = graph
                .find_edge(node.id, EdgeType::Internal, "second")
                .expect("ConcatString must have second");

            format!(
                "{} + {}",
                minimal_node_repr(first, graph),
                minimal_node_repr(second, graph)
            )
        }
        NodeType::SlicedString => {
            let parent = graph
                .find_edge(node.id, EdgeType::Internal, "parent")
                .expect("SlicedString must have parent");

            format!("<slice of {}>", minimal_node_repr(parent, graph))
        }
        NodeType::Number => "<a number>".to_string(),
        NodeType::BigInt => "<a bigint>".to_string(),
        NodeType::Closure => format!("function {}()", node.name()),
        NodeType::Symbol => match graph.find_edge(node.id, EdgeType::Internal, "name") {
            Some(name) => format!("symbol {}", minimal_node_repr(name, graph)),
            None => "unnamed symbol".to_string(),
        },
        NodeType::Object => {
            if graph
                .find_edge(node.id, EdgeType::Internal, "elements")
                .is_some()
            {
                // It's an array or array-like, format like an array
                return format!(
                    "{} [ {} ]",
                    node.name(),
                    graph
                        .out_edges(node.id)
                        .filter(|e| e.typ() == EdgeType::Element)
                        .map(|e| minimal_node_repr(e.to_node(), graph))
                        .join(", ")
                );
            }

            // Format as a regular object
            format!(
                "{} {{ {} }}",
                node.name(),
                graph
                    .out_edges(node.id)
                    .filter(|e| e.typ() == EdgeType::Property)
                    .map(|e| e.name_or_index().to_string())
                    .join(", ")
            )
        }
        _ => format!("{}:{}", node.typ_str(), node.print_safe_name(30)),
    }
}

pub fn detailed_node_repr(node: NodeId, graph: &V8HeapGraph) -> String {
    let node = graph.node(node);

    let mut ret = String::new();

    match node.typ() {
        NodeType::String => print_safe(node.name(), 50).to_string(),
        NodeType::Synthetic => {
            let _ = writeln!(&mut ret, "{}\n", node.name());
            print_edges(&mut ret, node.id, graph);
            ret
        }
        NodeType::ConcatString => {
            let first = graph
                .find_edge(node.id, EdgeType::Internal, "first")
                .expect("ConcatString must have first");
            let second = graph
                .find_edge(node.id, EdgeType::Internal, "second")
                .expect("ConcatString must have second");

            format!(
                "{} + {}",
                minimal_node_repr(first, graph),
                minimal_node_repr(second, graph)
            )
        }
        NodeType::SlicedString => {
            let parent = graph
                .find_edge(node.id, EdgeType::Internal, "parent")
                .expect("SlicedString must have parent");

            format!("<slice of {}>", minimal_node_repr(parent, graph))
        }
        NodeType::Number => "<a number>".to_string(),
        NodeType::BigInt => "<a bigint>".to_string(),
        NodeType::Closure => {
            let _ = writeln!(&mut ret, "function {}()\n", node.name());
            print_edges(&mut ret, node.id, graph);
            ret
        }
        NodeType::Symbol => match graph.find_edge(node.id, EdgeType::Internal, "name") {
            Some(name) => format!("symbol {}", minimal_node_repr(name, graph)),
            None => "unnamed symbol".to_string(),
        },
        NodeType::Object => {
            if graph
                .find_edge(node.id, EdgeType::Internal, "elements")
                .is_some()
            {
                let elements = graph
                    .out_edges(node.id)
                    .filter(|e| e.typ() == EdgeType::Element)
                    .map(|e| minimal_node_repr(e.to_node(), graph))
                    .collect_vec();

                let _ = writeln!(&mut ret, "{} ({} elements)\n", node.name(), elements.len());
                for el in elements {
                    let _ = writeln!(&mut ret, " - {}", el);
                }
                return ret;
            }

            let _ = writeln!(&mut ret, "{}\n", node.name());
            for edge in graph
                .out_edges(node.id)
                .filter(|e| e.typ() == EdgeType::Property)
            {
                let _ = writeln!(
                    &mut ret,
                    "  {}: {}",
                    edge.name_or_index(),
                    minimal_node_repr(edge.to_node(), graph)
                );
            }
            ret
        }
        _ => format!("{}:{}", node.typ_str(), node.print_safe_name(30)),
    }
}

pub fn print_edges<F: std::fmt::Write>(f: &mut F, node: NodeId, graph: &V8HeapGraph) {
    for edge in graph.out_edges(node) {
        let _ = writeln!(
            f,
            "  -[{}:{}]-> {}  {}",
            edge.typ_str(),
            edge.name_or_index(),
            edge.to_node(),
            minimal_node_repr(edge.to_node(), graph),
        );
    }
}

pub fn format_retention_paths<F: std::fmt::Write>(
    f: &mut F,
    node: NodeId,
    paths: &RootPaths,
    graph: &V8HeapGraph,
) -> std::fmt::Result {
    for path in paths.paths_to(node, graph) {
        for edge in path.edges(graph) {
            fmt_edge(f, &edge)?;
        }
        writeln!(f)?;
    }
    Ok(())
}

fn fmt_edge<F: std::fmt::Write>(f: &mut F, edge: &Edge<'_>) -> std::fmt::Result {
    match edge.typ() {
        EdgeType::Property => write!(f, ".{}", edge.name_or_index()),
        EdgeType::Element => write!(f, "[{}]", edge.index()),
        EdgeType::Internal => write!(f, "(internal/{})", edge.name_or_index()),
        EdgeType::Context => write!(f, "(context/{})", edge.name_or_index()),
        EdgeType::Shortcut => write!(f, "(shortcut/{})", edge.name_or_index()),
        EdgeType::Weak => write!(f, "(weak/{})", edge.name_or_index()),
        EdgeType::Hidden => write!(f, "(hidden/{})", edge.name_or_index()),
    }
}

pub fn print_dominator_tree(tree: &DominatorTree, graph: &V8HeapGraph) {
    print_dominator_node(0, tree, graph, 0);
}

fn print_dominator_node(node_id: NodeId, tree: &DominatorTree, graph: &V8HeapGraph, depth: usize) {
    let node = graph.node(node_id);
    let retained_size = tree.retained_sizes[node_id as usize];

    println!(
        "{}[{}]  {}  ({})",
        "    ".repeat(depth),
        node.stable_id(),
        minimal_node_repr(node.id, graph),
        format_bytes(retained_size),
    );

    if let Some(mut children) = tree.children.get(&node_id).cloned() {
        // Sort by retained sizes descending
        children.sort_by_key(|node| -(tree.retained_sizes[*node as usize] as i64));

        // Some nodes we're going to hide
        children.retain(|node| {
            !matches!(
                graph.node(*node).typ(),
                NodeType::Hidden
                    | NodeType::ObjectShape
                    | NodeType::ConcatString
                    | NodeType::SlicedString
                    | NodeType::Code
                    | NodeType::Array
            )
        });

        for child in &children[0..20.min(children.len())] {
            print_dominator_node(*child, tree, graph, depth + 1);
        }
    }
}

fn show_node(node: Node<'_>) -> String {
    node.graph
        .out_edges(node.id)
        .map(|e| format!("{} {} {}", e.typ_str(), e.name_or_index(), e.to_node()))
        .join(", ")
}
