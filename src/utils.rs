/// Escapes special characters in strings for display
pub fn escape_string(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '\n' => vec!['\\', 'n'],
            '\r' => vec!['\\', 'r'],
            '\t' => vec!['\\', 't'],
            '\\' => vec!['\\', '\\'],
            '"' => vec!['\\', '"'],
            c if c.is_control() => format!("\\u{:04x}", c as u32).chars().collect(),
            c => vec![c],
        })
        .collect()
}

/// Formats bytes into human-readable format
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Formats a node name for display (used in retention paths and reports)
pub fn format_node_name(name: &str) -> String {
    name.to_string()
}

/// Formats an edge name for display (quotes and escapes if needed)
pub fn format_edge_name(edge_name: &str) -> String {
    // Check if it looks like a string value (not a property name)
    // Property names are typically alphanumeric identifiers
    if edge_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') {
        edge_name.to_string()
    } else {
        // Format as escaped string
        format!("\"{}\"", escape_string(edge_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_string("tab\there"), "tab\\there");
        assert_eq!(escape_string("quote\"test"), "quote\\\"test");
        assert_eq!(escape_string("backslash\\test"), "backslash\\\\test");
        assert_eq!(escape_string("normal text"), "normal text");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 bytes");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }
}
