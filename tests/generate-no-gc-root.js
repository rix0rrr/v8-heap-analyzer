// Test: Generate heap snapshot with 1000 duplicate strings NOT in GC root
const v8 = require('v8');
const fs = require('fs');
const path = require('path');

// Create strings in a function scope that will be gone
function createStrings() {
    const duplicateArray = [];
    for (let i = 0; i < 1000; i++) {
        const str = 'y'.repeat(100);
        duplicateArray.push(str);
    }
    // Function returns, array goes out of scope
}

// Call the function
createStrings();

// Force garbage collection if available
if (global.gc) {
    global.gc();
    console.log('Forced garbage collection');
}

// Generate snapshot after strings should be collected
const snapshotPath = path.join(__dirname, 'fixtures', 'test-no-gc-root.heapsnapshot');
v8.writeHeapSnapshot(snapshotPath);

console.log(`Snapshot written to: ${snapshotPath}`);
console.log(`Created 1000 copies of dynamically generated strings in function scope`);
console.log(`String length: 100 characters`);
console.log(`NOT stored in GC root - should be collected`);
