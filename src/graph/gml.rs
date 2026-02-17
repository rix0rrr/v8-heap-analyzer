use std::{io::BufWriter, path::Path};

use crate::utils::escape_string;

pub fn write_gml_file(
    filename: &Path,
    graph: &super::v8_heap_graph::V8HeapGraph,
) -> anyhow::Result<()> {
    let f = std::fs::File::create(filename)?;
    write_gml(&mut BufWriter::new(f), graph)?;
    Ok(())
}

pub fn write_gml<F: std::io::Write>(
    f: &mut F,
    graph: &super::v8_heap_graph::V8HeapGraph,
) -> std::io::Result<()> {
    writeln!(f, r#"graph ["#)?;
    for node_id in graph.iter_nodes() {
        let node = graph.node(node_id);

        writeln!(
            f,
            "  node [\n    id {}\n    label \"{}:{}\"\n  ]",
            node_id,
            node.typ_str(),
            xml_quote(&node.print_safe_name(30)),
        )?;
    }
    for edge_id in graph.iter_edges() {
        let edge = graph.edge(edge_id);

        writeln!(
            f,
            "  edge [\n    source {}\n    target {}\n    label \"{}:{}\"\n  ]",
            edge.from_node,
            edge.to_node(),
            edge.typ_str(),
            xml_quote(&escape_string(&format!("{}", edge.name_or_index()))),
        )?;
    }

    writeln!(f, "]")?;

    Ok(())
}

fn xml_quote(x: &str) -> String {
    x.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
