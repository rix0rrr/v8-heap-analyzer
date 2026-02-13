use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SnapshotMetadata {
    pub node_fields: Vec<String>,
    pub node_types: Vec<serde_json::Value>,
    pub edge_fields: Vec<String>,
    pub edge_types: Vec<serde_json::Value>,
    #[serde(default)]
    pub node_count: usize,
    #[serde(default)]
    pub edge_count: usize,
}

impl SnapshotMetadata {
    pub fn node_field_count(&self) -> usize {
        self.node_fields.len()
    }

    pub fn edge_field_count(&self) -> usize {
        self.edge_fields.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metadata() {
        let json = r#"{
            "node_fields": ["type", "name", "id", "self_size", "edge_count"],
            "node_types": [
                ["hidden", "array", "string", "object"],
                "string",
                "number",
                "number",
                "number"
            ],
            "edge_fields": ["type", "name_or_index", "to_node"],
            "edge_types": [
                ["context", "element", "property", "internal"],
                "string_or_number",
                "node"
            ],
            "node_count": 1000,
            "edge_count": 2000
        }"#;

        let meta: SnapshotMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.node_fields.len(), 5);
        assert_eq!(meta.edge_fields.len(), 3);
        assert_eq!(meta.node_count, 1000);
        assert_eq!(meta.edge_count, 2000);
        assert_eq!(meta.node_field_count(), 5);
        assert_eq!(meta.edge_field_count(), 3);
    }
}
