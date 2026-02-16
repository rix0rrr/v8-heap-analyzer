use std::path::PathBuf;

// Import the analyze function from main

#[test]
fn test_find_string_duplicates_in_snapshot() {
    let snapshot_path = PathBuf::from("tests/fixtures/test-string-duplicates.heapsnapshot");

    if !snapshot_path.exists() {
        panic!("Test snapshot not found. Run: node tests/generate-string-duplicates.js");
    }
}
