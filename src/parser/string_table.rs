pub struct StringTable {
    strings: Vec<String>,
}

impl StringTable {
    pub fn new(strings: Vec<String>) -> Self {
        Self { strings }
    }

    pub fn get(&self, idx: u32) -> Option<&str> {
        self.strings.get(idx as usize).map(|s| s.as_str())
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_table_get() {
        let table = StringTable::new(vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ]);

        assert_eq!(table.get(0), Some("first"));
        assert_eq!(table.get(1), Some("second"));
        assert_eq!(table.get(2), Some("third"));
        assert_eq!(table.get(3), None);
    }

    #[test]
    fn test_string_table_len() {
        let table = StringTable::new(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(table.len(), 2);
        assert!(!table.is_empty());

        let empty = StringTable::new(vec![]);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }
}
