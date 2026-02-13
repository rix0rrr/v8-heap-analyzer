use std::path::PathBuf;

// Import the analyze function from main
use v8_heap_analyzer::{analyze_snapshot, AnalysisResults};

#[test]
fn test_find_string_duplicates_in_snapshot() {
    let snapshot_path = PathBuf::from("tests/fixtures/test-string-duplicates.heapsnapshot");
    
    if !snapshot_path.exists() {
        panic!("Test snapshot not found. Run: node tests/generate-string-duplicates.js");
    }
    
    // Analyze snapshot directly
    let results = analyze_snapshot(&snapshot_path, false)
        .expect("Failed to analyze snapshot");
    
    // Verify we found duplicate groups
    assert!(!results.duplicate_groups.is_empty(), "No duplicate groups found");
    
    // Find our test string (1000 x's)
    let found_test_string = results.duplicate_groups.iter().any(|group| {
        group.object_type.starts_with("xxxx") && group.count == 1000
    });
    
    assert!(found_test_string, 
        "Test string (1000 duplicates of x's) not found in duplicate groups"
    );
    
    // Verify we found exactly 1000 duplicates
    let test_group = results.duplicate_groups.iter()
        .find(|group| group.object_type.starts_with("xxxx") && group.count == 1000)
        .expect("Test string group not found");
    
    assert_eq!(test_group.count, 1000, "Expected exactly 1000 duplicates, found {}", test_group.count);
    
    println!("✓ Found exactly {} duplicates of 100-character string", test_group.count);
    
    // Verify it's in the top groups (should be high impact)
    assert!(test_group.total_wasted > 20000, 
        "Expected significant memory waste, found {} bytes", 
        test_group.total_wasted
    );
    
    println!("✓ Total wasted memory: {} bytes", test_group.total_wasted);
}

#[test]
fn test_find_object_duplicates_in_snapshot() {
    let snapshot_path = PathBuf::from("tests/fixtures/test-object-duplicates.heapsnapshot");
    
    if !snapshot_path.exists() {
        panic!("Test snapshot not found. Run: node tests/generate-object-duplicates.js");
    }
    
    // Analyze snapshot directly
    let results = analyze_snapshot(&snapshot_path, false)
        .expect("Failed to analyze snapshot");
    
    // Verify we found duplicate groups
    assert!(!results.duplicate_groups.is_empty(), "No duplicate groups found");
    
    // Find groups with 1001 duplicates (1000 + 1 original)
    let groups_with_1001 = results.duplicate_groups.iter()
        .filter(|group| group.count == 1001)
        .count();
    
    assert!(groups_with_1001 >= 3, 
        "Expected at least 3 groups with 1001 duplicates (main + nested objects), found {}", 
        groups_with_1001
    );
    
    println!("✓ Found {} groups with 1001 duplicates (complex objects with nesting)", groups_with_1001);
    
    // Verify total wasted memory is significant
    let total_wasted: u64 = results.duplicate_groups.iter()
        .filter(|group| group.count == 1001)
        .map(|group| group.total_wasted)
        .sum();
    
    assert!(total_wasted > 100000, 
        "Expected significant memory waste from objects, found {} bytes", 
        total_wasted
    );
    
    println!("✓ Total wasted memory from duplicate objects: {} bytes", total_wasted);
}

#[test]
fn test_unicode_strings_no_crash() {
    let snapshot_path = PathBuf::from("tests/fixtures/test-unicode-duplicates.heapsnapshot");
    
    if !snapshot_path.exists() {
        panic!("Test snapshot not found. Run: node tests/generate-unicode-duplicates.js");
    }
    
    // Analyze snapshot - should not crash on unicode
    let results = analyze_snapshot(&snapshot_path, false)
        .expect("Analyzer crashed on unicode");
    
    // Verify we got results
    assert!(!results.duplicate_groups.is_empty(), "No duplicate groups found");
    
    // Verify sample values don't cause panics (they contain unicode)
    for group in results.duplicate_groups.iter().take(10) {
        if let Some(ref sample) = group.sample_value {
            // Just accessing the sample value - if it panicked on unicode, we'd crash here
            assert!(!sample.is_empty(), "Sample value should not be empty");
        }
    }
    
    println!("✓ Successfully analyzed snapshot with unicode strings");
}

#[test]
fn test_multiple_retention_paths_from_js() {
    let snapshot_path = PathBuf::from("tests/fixtures/test-multiple-paths.heapsnapshot");
    
    if !snapshot_path.exists() {
        panic!("Test snapshot not found. Run: node tests/generate-multiple-paths.js");
    }
    
    // Analyze snapshot
    let results = analyze_snapshot(&snapshot_path, false)
        .expect("Failed to analyze snapshot");
    
    // The shared object should appear in retention paths
    // Look for objects that have multiple retention paths
    let objects_with_multiple_paths = results.retention_paths.iter()
        .filter(|(_, paths)| paths.len() >= 2)
        .count();
    
    assert!(objects_with_multiple_paths > 0, 
        "Expected to find at least one object with multiple retention paths, found none"
    );
    
    println!("✓ Found {} objects with multiple retention paths", objects_with_multiple_paths);
    
    // Verify the paths are actually different (start from different roots)
    for (node_id, paths) in results.retention_paths.iter() {
        if paths.len() >= 2 {
            let root_nodes: std::collections::HashSet<_> = paths.iter()
                .map(|p| p.nodes[0])
                .collect();
            
            assert!(root_nodes.len() >= 2, 
                "Object {} has {} paths but they start from the same root", 
                node_id, paths.len()
            );
            
            println!("✓ Object {} has {} paths from {} different GC roots", 
                node_id, paths.len(), root_nodes.len());
            break; // Just verify one object
        }
    }
}
