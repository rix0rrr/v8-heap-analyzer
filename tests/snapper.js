const path = require('path');
const v8 = require('v8');

let script = process.argv[2];
if (!script) {
  throw new Error('Usage: snapper.js <SCRIPT>');
}

script = script.replace(/\.js$/, '');

// Run that script
require(`./scripts/${script}.js`);

// Generate snapshot
const snapshotPath = path.join(__dirname, 'fixtures', `${script}.heapsnapshot`);
v8.writeHeapSnapshot(snapshotPath);
