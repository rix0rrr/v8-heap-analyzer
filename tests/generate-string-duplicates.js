// Test: Generate heap snapshot with 1000 duplicate strings (non-interned)
const v8 = require('v8');
const fs = require('fs');
const path = require('path');

// Create 1000 copies of dynamically generated strings that can't be interned
const duplicateArray = [];

for (let i = 0; i < 1000; i++) {
    // Create string dynamically to avoid interning
    const str = 'x'.repeat(100);
    duplicateArray.push(str);
}

// Keep in global scope (GC root)
global.testDuplicateArray = duplicateArray;

// Generate snapshot
const snapshotPath = path.join(__dirname, 'fixtures', 'test-string-duplicates.heapsnapshot');
v8.writeHeapSnapshot(snapshotPath);

console.log(`Snapshot written to: ${snapshotPath}`);
console.log(`Created ${duplicateArray.length} copies of dynamically generated strings`);
console.log(`String length: 100 characters`);
