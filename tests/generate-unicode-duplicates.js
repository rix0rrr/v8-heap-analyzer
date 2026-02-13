// Test: Generate heap snapshot with unicode string duplicates
const v8 = require('v8');
const fs = require('fs');
const path = require('path');

// Create 1000 copies of strings with unicode characters (dynamically to avoid interning)
const duplicateArray = [];

// Create strings dynamically to avoid interning
for (let i = 0; i < 200; i++) {
    // Chinese characters - create dynamically
    const chinese = String.fromCharCode(20013, 25991) + '(' + String.fromCharCode(31616, 20307) + ')';
    duplicateArray.push(chinese);
    
    // Japanese - create dynamically
    const japanese = String.fromCharCode(12371, 12435, 12395, 12385, 12399);
    duplicateArray.push(japanese);
    
    // Emoji - create dynamically
    const emoji = String.fromCodePoint(0x1F389, 0x1F680, 0x1F4BB);
    duplicateArray.push(emoji);
    
    // Long unicode string that will need truncation
    const longUnicode = '[{"code":"en","status":"GA","label":"English (US)"},{"code":"zh-CN","status":"GA","label":"' + 
                        String.fromCharCode(20013, 25991) + '(' + String.fromCharCode(31616, 20307) + ')"}]';
    duplicateArray.push(longUnicode);
}

// Keep in global scope (GC root)
global.testUnicodeArray = duplicateArray;

// Generate snapshot
const snapshotPath = path.join(__dirname, 'fixtures', 'test-unicode-duplicates.heapsnapshot');
v8.writeHeapSnapshot(snapshotPath);

console.log(`Snapshot written to: ${snapshotPath}`);
console.log(`Created ${duplicateArray.length} copies of unicode strings`);
console.log(`String types: Chinese, Japanese, Emoji, Long mixed`);
