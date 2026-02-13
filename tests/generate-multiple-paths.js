// Test: Generate heap snapshot with object reachable from multiple GC roots
const v8 = require('v8');
const path = require('path');

// Create a shared object
const sharedObject = {
    id: 'shared-target',
    data: 'This object is referenced from multiple places'
};

// Reference it from multiple global variables (GC roots)
global.reference1 = {
    name: 'first-root',
    target: sharedObject
};

global.reference2 = {
    name: 'second-root',
    target: sharedObject
};

global.reference3 = {
    name: 'third-root',
    target: sharedObject
};

// Generate snapshot
const snapshotPath = path.join(__dirname, 'fixtures', 'test-multiple-paths.heapsnapshot');
v8.writeHeapSnapshot(snapshotPath);

console.log(`Snapshot written to: ${snapshotPath}`);
console.log(`Created object reachable from 3 different GC roots`);
console.log(`Shared object ID: ${sharedObject.id}`);
