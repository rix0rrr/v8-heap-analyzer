// Generate heap snapshot with duplicate strings
const v8 = require('v8');
const fs = require('fs');
const path = require('path');

// Create many duplicate strings
const duplicates = [];
for (let i = 0; i < 1000; i++) {
    duplicates.push("duplicate-string-value");
    duplicates.push({ name: "test", value: 42 });
}

// Keep them in memory
global.testDuplicates = duplicates;

// Generate snapshot
const snapshotPath = path.join(__dirname, 'fixtures', 'test-duplicates.heapsnapshot');
v8.writeHeapSnapshot(snapshotPath);

console.log(`Snapshot written to: ${snapshotPath}`);
console.log(`Created ${duplicates.length} items with duplicates`);
