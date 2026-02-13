use std::process::Command;
use std::path::PathBuf;

#[test]
fn test_find_string_duplicates_in_snapshot() {
    // Path to test snapshot
    let snapshot_path = PathBuf::from("tests/fixtures/test-string-duplicates.heapsnapshot");
    
    if !snapshot_path.exists() {
        panic!("Test snapshot not found. Run: node tests/generate-string-duplicates.js");
    }
    
    // Run analyzer
    let output_path = PathBuf::from("tests/fixtures/test-output.json");
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--release",
            "--",
            "-i",
            snapshot_path.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("Failed to run analyzer");
    
    assert!(output.status.success(), "Analyzer failed: {:?}", String::from_utf8_lossy(&output.stderr));
    
    // Read and parse JSON output
    let report_json = std::fs::read_to_string(&output_path)
        .expect("Failed to read output file");
    
    let report: serde_json::Value = serde_json::from_str(&report_json)
        .expect("Failed to parse JSON");
    
    // Verify we found duplicate groups
    let duplicate_groups = report["duplicate_groups"].as_array()
        .expect("No duplicate_groups in report");
    
    assert!(!duplicate_groups.is_empty(), "No duplicate groups found");
    
    // Find our test string (100 x's, but may be truncated in output) in the duplicates
    let found_test_string = duplicate_groups.iter().any(|group| {
        group["object_type"].as_str().map_or(false, |s| {
            s.starts_with("xxxx") && group["count"].as_u64() == Some(1000)
        })
    });
    
    assert!(found_test_string, 
        "Test string (1000 duplicates of x's) not found in duplicate groups"
    );
    
    // Verify we found exactly 1000 duplicates
    let test_group = duplicate_groups.iter()
        .find(|group| {
            group["object_type"].as_str().map_or(false, |s| {
                s.starts_with("xxxx") && group["count"].as_u64() == Some(1000)
            })
        })
        .expect("Test string group not found");
    
    let count = test_group["count"].as_u64().expect("No count field");
    
    // Should find exactly 1000 duplicates
    assert_eq!(count, 1000, "Expected exactly 1000 duplicates, found {}", count);
    
    println!("✓ Found exactly {} duplicates of 100-character string", count);
    
    // Verify it's in the top groups (should be high impact)
    let total_wasted = test_group["total_wasted"].as_u64().expect("No total_wasted field");
    assert!(total_wasted > 20000, "Expected significant memory waste, found {} bytes", total_wasted);
    
    println!("✓ Total wasted memory: {} bytes", total_wasted);
    
    // Cleanup
    let _ = std::fs::remove_file(output_path);
}
