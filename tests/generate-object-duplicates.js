// Test: Generate heap snapshot with 1000 duplicate complex objects
const v8 = require('v8');
const fs = require('fs');
const path = require('path');

// Create 1000 copies of identical complex objects
const duplicateArray = [];

for (let i = 0; i < 1000; i++) {
    const obj = {
        id: 42,
        name: "test-object",
        value: 123.456,
        active: true,
        tags: ["tag1", "tag2", "tag3"],
        metadata: {
            created: "2024-01-01",
            modified: "2024-01-02",
            author: "test-user",
            version: 1,
            flags: {
                enabled: true,
                visible: true,
                locked: false
            }
        },
        config: {
            timeout: 5000,
            retries: 3,
            endpoint: "https://example.com",
            headers: {
                "Content-Type": "application/json",
                "Authorization": "Bearer token123"
            }
        },
        stats: {
            count: 100,
            total: 5000,
            average: 50
        }
    };
    
    duplicateArray.push(obj);
}

// Keep in global scope (GC root)
global.testDuplicateObjects = duplicateArray;

// Generate snapshot
const snapshotPath = path.join(__dirname, 'fixtures', 'test-object-duplicates.heapsnapshot');
v8.writeHeapSnapshot(snapshotPath);

console.log(`Snapshot written to: ${snapshotPath}`);
console.log(`Created ${duplicateArray.length} copies of complex objects`);
console.log(`Each object has 8 top-level keys with nested objects`);
