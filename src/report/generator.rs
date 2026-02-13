use crate::analysis::duplicates::DuplicateGroup;
use crate::analysis::hidden_classes::HiddenClassGroup;
use crate::graph::CompactGraph;
use crate::paths::RetentionPath;
use crate::types::NodeId;
use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;

pub struct ReportGenerator<'a> {
    graph: &'a CompactGraph,
    duplicate_groups: Vec<DuplicateGroup>,
    hidden_class_groups: Vec<HiddenClassGroup>,
    retention_paths: HashMap<NodeId, Vec<RetentionPath>>,
}

impl<'a> ReportGenerator<'a> {
    pub fn new(
        graph: &'a CompactGraph,
        duplicate_groups: Vec<DuplicateGroup>,
        hidden_class_groups: Vec<HiddenClassGroup>,
        retention_paths: HashMap<NodeId, Vec<RetentionPath>>,
    ) -> Self {
        Self {
            graph,
            duplicate_groups,
            hidden_class_groups,
            retention_paths,
        }
    }

    pub fn generate_text_report(&self, output: &mut dyn Write, top_n: usize) -> Result<()> {
        writeln!(output, "V8 Heap Snapshot Analysis")?;
        writeln!(output, "==========================")?;
        writeln!(output)?;
        
        // Summary
        let total_objects = self.graph.node_count();
        let total_wasted: u64 = self.duplicate_groups.iter().map(|g| g.total_wasted).sum();
        
        writeln!(output, "Summary:")?;
        writeln!(output, "- Total Objects: {}", total_objects)?;
        writeln!(output, "- Duplicate Groups Found: {}", self.duplicate_groups.len())?;
        writeln!(output, "- Total Wasted Memory: {} bytes", total_wasted)?;
        writeln!(output)?;
        
        // Top N duplicate groups
        writeln!(output, "Top {} Duplicate Groups (by memory impact):", top_n)?;
        writeln!(output, "-------------------------------------------")?;
        writeln!(output)?;
        
        for (i, group) in self.duplicate_groups.iter().take(top_n).enumerate() {
            writeln!(output, "{}. {}", i + 1, group.object_type)?;
            writeln!(output, "   Count: {} duplicates", group.count)?;
            writeln!(output, "   Size: {} bytes each", group.size_per_object)?;
            writeln!(output, "   Total Wasted: {} bytes", group.total_wasted)?;
            
            if let Some(paths) = self.retention_paths.get(&group.representative) {
                if let Some(path) = paths.first() {
                    writeln!(output, "   Retention Path:")?;
                    self.format_path(output, path)?;
                }
            }
            writeln!(output)?;
        }
        
        // Top N hidden class groups
        writeln!(output, "Top {} Object Types by Hidden Class Memory:", top_n)?;
        writeln!(output, "--------------------------------------------")?;
        writeln!(output)?;
        
        for (i, group) in self.hidden_class_groups.iter().take(top_n).enumerate() {
            writeln!(output, "{}. {}", i + 1, group.object_type)?;
            writeln!(output, "   Hidden Classes: {}", group.hidden_class_count)?;
            writeln!(output, "   Total Memory: {} bytes", group.total_hidden_class_memory)?;
            writeln!(output)?;
        }
        
        Ok(())
    }

    pub fn generate_json_report(&self, output: &mut dyn Write, top_n: usize) -> Result<()> {
        let report = JsonReport {
            summary: Summary {
                total_objects: self.graph.node_count(),
                duplicate_groups: self.duplicate_groups.len(),
                total_wasted: self.duplicate_groups.iter().map(|g| g.total_wasted).sum(),
            },
            duplicate_groups: self.duplicate_groups.iter().take(top_n).cloned().collect(),
            hidden_class_groups: self.hidden_class_groups.iter().take(top_n).cloned().collect(),
        };
        
        serde_json::to_writer_pretty(output, &report)?;
        Ok(())
    }

    fn format_path(&self, output: &mut dyn Write, path: &RetentionPath) -> Result<()> {
        for (i, &node_id) in path.nodes.iter().enumerate() {
            let name = self.graph.node_name(node_id).unwrap_or("unknown");
            writeln!(output, "     {} {}", "  ".repeat(i), name)?;
            
            if i < path.edge_names.len() {
                writeln!(output, "     {}   .{}", "  ".repeat(i), path.edge_names[i])?;
            }
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct JsonReport {
    summary: Summary,
    duplicate_groups: Vec<DuplicateGroup>,
    hidden_class_groups: Vec<HiddenClassGroup>,
}

#[derive(Serialize)]
struct Summary {
    total_objects: usize,
    duplicate_groups: usize,
    total_wasted: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::StringTable;
    use std::sync::Arc;

    #[test]
    fn test_generate_text_report() {
        let strings = vec!["".to_string(), "test".to_string()];
        let string_table = Arc::new(StringTable::new(strings));
        let graph = CompactGraph::new(string_table);
        
        let groups = vec![DuplicateGroup {
            hash: 123,
            object_type: "String".to_string(),
            count: 10,
            size_per_object: 48,
            total_wasted: 432,
            representative: 0,
            node_ids: vec![0, 1, 2],
        }];
        
        let generator = ReportGenerator::new(&graph, groups, vec![], HashMap::new());
        let mut output = Vec::new();
        generator.generate_text_report(&mut output, 10).unwrap();
        
        let report = String::from_utf8(output).unwrap();
        assert!(report.contains("V8 Heap Snapshot Analysis"));
        assert!(report.contains("String"));
        assert!(report.contains("432 bytes"));
    }
}
