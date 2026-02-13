# V8 Heap Snapshot Analyzer

A memory-efficient Rust-based analyzer for V8 heap snapshots that identifies duplicate objects and hidden class memory consumption.

## Features

- **Duplicate Detection**: Find exact duplicate strings and objects
- **Hidden Class Analysis**: Identify object types with excessive shape variations
- **Retention Path Analysis**: Trace paths from GC roots to understand memory retention
- **Memory Efficient**: Analyze 5-10GB snapshots using ~6-7GB memory
- **Multiple Output Formats**: Text (human-readable) and JSON (machine-readable)
- **Progress Reporting**: Real-time feedback during analysis

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/v8-heap-analyzer`.

## Usage

### Basic Usage

```bash
# Generate text report
v8-heap-analyzer -i snapshot.heapsnapshot -o report.txt

# Generate JSON report
v8-heap-analyzer -i snapshot.heapsnapshot -o report.json --format json
```

### Options

- `-i, --input <FILE>`: Input heap snapshot file (required)
- `-o, --output <FILE>`: Output report file (required)
- `-f, --format <FORMAT>`: Output format: `text` or `json` (default: `text`)
- `--include-hidden-classes`: Include hidden classes in duplicate detection

### Generating Heap Snapshots

#### Node.js

```javascript
const v8 = require('v8');
v8.writeHeapSnapshot('snapshot.heapsnapshot');
```

#### Chrome DevTools

1. Open DevTools (F12)
2. Go to Memory tab
3. Select "Heap snapshot"
4. Click "Take snapshot"
5. Right-click and "Save as..."

## Output

### Text Report

```
V8 Heap Snapshot Analysis
==========================

Summary:
- Total Objects: 44,194
- Duplicate Groups Found: 181
- Total Wasted Memory: 607,080 bytes

Top 10 Duplicate Groups (by memory impact):
-------------------------------------------

1. String: "duplicate-value"
   Count: 16 duplicates
   Size: 35,176 bytes each
   Total Wasted: 527,640 bytes
   Retention Path:
     Window (GC root)
       .document
     HTMLDocument
       .cache
     ...
```

### JSON Report

```json
{
  "summary": {
    "total_objects": 44194,
    "duplicate_groups": 181,
    "total_wasted": 607080
  },
  "duplicate_groups": [
    {
      "object_type": "String",
      "count": 16,
      "size_per_object": 35176,
      "total_wasted": 527640,
      "representative": 11413,
      "node_ids": [11413, 11415, ...]
    }
  ],
  "hidden_class_groups": [...]
}
```

## How It Works

1. **Parse Snapshot**: Reads V8 heap snapshot JSON format
2. **Build Graph**: Constructs memory-efficient graph representation
3. **Analyze Duplicates**: Uses hash-based detection to find identical objects
4. **Analyze Hidden Classes**: Groups objects by type and calculates hidden class memory
5. **Find Retention Paths**: BFS from GC roots to understand why objects are retained
6. **Generate Report**: Outputs actionable insights for developers

## Architecture

- **Compact Graph**: Structure of Arrays layout for 60% memory savings
- **Streaming Parser**: Processes large files without loading entirely into memory
- **Hash-Based Detection**: O(n) duplicate detection with ahash
- **BFS Path Finding**: Efficient shortest path algorithm for retention analysis

## Testing

```bash
# Run all tests (unit + integration)
cargo test

# Generate test snapshots
node tests/generate-snapshot.js
node tests/generate-string-duplicates.js
node tests/generate-object-duplicates.js

# Analyze test snapshot
./target/release/v8-heap-analyzer \
  -i tests/fixtures/test-duplicates.heapsnapshot \
  -o test-report.txt
```

### Test Coverage

- **15 unit tests**: Core functionality (parser, graph, analysis, paths, reports)
- **2 integration tests**: End-to-end validation with real heap snapshots
  - **String duplicates**: Generates 1000 duplicate strings using `'x'.repeat(100)`, verifies analyzer finds exactly 1000 duplicates
  - **Object duplicates**: Generates 1000 duplicate complex objects with nested structures (8 top-level keys, multiple nested objects), verifies analyzer finds all duplicate objects including nested ones
  - Validates memory waste calculation and retention paths

## Performance

- **Test snapshot** (44K nodes): ~2-3 seconds
- **Large snapshot** (10GB, 100M nodes): ~3-5 minutes
- **Memory usage**: ~6.5GB for 10GB snapshot

## Project Structure

```
src/
├── main.rs              # CLI entry point
├── types.rs             # Core type definitions
├── parser/              # Snapshot parsing
│   ├── metadata.rs
│   ├── string_table.rs
│   └── mod.rs
├── graph/               # Graph data structures
│   ├── compact.rs
│   ├── builder.rs
│   └── mod.rs
├── analysis/            # Analysis algorithms
│   ├── duplicates.rs
│   ├── hidden_classes.rs
│   └── mod.rs
├── paths/               # Retention path finding
│   ├── finder.rs
│   └── mod.rs
└── report/              # Report generation
    ├── generator.rs
    └── mod.rs
```

## Documentation

See `planning/` directory for:
- Requirements clarification
- Research findings
- Detailed design document
- Implementation plan
- Progress checkpoints

## License

MIT

## Contributing

Contributions welcome! Please ensure:
- All tests pass (`cargo test`)
- Code follows Rust conventions (`cargo fmt`, `cargo clippy`)
- New features include tests
