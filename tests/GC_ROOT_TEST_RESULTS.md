# GC Root Test Results

## Experiment: Duplicate Strings With and Without GC Roots

### Test 1: Strings Stored in GC Root (Global Variable)

**Setup:**
```javascript
const duplicateArray = [];
for (let i = 0; i < 1000; i++) {
    duplicateArray.push('x'.repeat(100));
}
global.testDuplicateArray = duplicateArray;  // Stored in GC root
v8.writeHeapSnapshot();
```

**Result:**
```
2. xxxxxxxx
   Count: 1000 duplicates
   Size: 24 bytes each
   Total Wasted: 23976 bytes
   Retention Path:
     (Stack roots)
       .30
     Array
       ...
```

✅ **All 1000 duplicates found** - strings are retained by the global variable

---

### Test 2: Strings NOT Stored in GC Root (Garbage Collected)

**Setup:**
```javascript
function createStrings() {
    const duplicateArray = [];
    for (let i = 0; i < 1000; i++) {
        duplicateArray.push('y'.repeat(100));
    }
    // Function returns, array goes out of scope
}
createStrings();
global.gc();  // Force garbage collection
v8.writeHeapSnapshot();
```

**Result:**
```
No 'yyyy' strings found in duplicate report
Total Objects: 45,146
Duplicate Groups Found: 192
```

❌ **Zero duplicates found** - strings were garbage collected before snapshot

---

## Conclusion

The V8 Heap Analyzer correctly identifies duplicates **only for objects that are retained in memory**. This is the expected and correct behavior because:

1. **Garbage collected objects don't waste memory** - they've already been freed
2. **Only retained objects matter** - these are the ones actually consuming memory
3. **Retention paths are essential** - they show WHY objects remain in memory

This validates that the analyzer is working correctly and providing actionable insights. It only reports duplicates that are actually wasting memory in the live heap, not objects that have already been collected.

### Key Insight

The duplicate detector finds duplicates that:
- ✅ Are reachable from GC roots
- ✅ Are actually consuming memory
- ✅ Have retention paths explaining why they exist

It does NOT report:
- ❌ Garbage collected objects
- ❌ Unreachable objects
- ❌ Objects that don't waste memory

This is exactly what developers need to fix memory issues!
