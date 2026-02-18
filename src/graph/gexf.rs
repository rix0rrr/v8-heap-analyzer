use std::{io::BufWriter, path::Path};

use crate::utils::escape_string;

pub fn write_gexf_file(
    filename: &Path,
    graph: &super::v8_heap_graph::V8HeapGraph,
) -> anyhow::Result<()> {
    let f = std::fs::File::create(filename)?;
    write_gexf(&mut BufWriter::new(f), graph)?;
    Ok(())
}

pub fn write_gexf<F: std::io::Write>(
    f: &mut F,
    graph: &super::v8_heap_graph::V8HeapGraph,
) -> std::io::Result<()> {
    writeln!(f, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    writeln!(f, r#"<gexf xmlns="http://gexf.net/1.2" version="1.2">"#)?;
    writeln!(f, r#"<graph mode="static" defaultedgetype="directed">"#)?;
    writeln!(f, r#"    <nodes>"#)?;
    for node_id in graph.iter_nodes() {
        let node = graph.node(node_id);

        writeln!(
            f,
            r#"        <node id="{}" label="{}:{}" />"#,
            node_id,
            node.typ_str(),
            xml_quote(&node.print_safe_name(30)),
        )?;
    }
    writeln!(f, r#"    </nodes>"#)?;
    writeln!(f, r#"    <edges>"#)?;

    for edge_id in graph.iter_edges() {
        let edge = graph.edge(edge_id);

        writeln!(
            f,
            r#"        <edge id="{}" source="{}" target="{}" label="{}:{} ({})" />"#,
            edge_id,
            edge.from_node(),
            edge.to_node(),
            edge.typ_str(),
            xml_quote(&escape_string(&format!("{}", edge.name_or_index()))),
            edge.index(),
        )?;
    }

    writeln!(f, r#"    </edges>"#)?;
    writeln!(f, "</graph>")?;
    writeln!(f, "</gexf>")?;

    Ok(())
}

fn xml_quote(x: &str) -> String {
    x.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
