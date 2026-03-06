v8-heap-analyzer
================

A different tool to analyze V8 heap dumps.

I wanted something slightly different from what Chrome DevTools gave me, so I made my own.

How to use
----------

```ts
global.gc();
require('v8').writeHeapSnapshot(/* optional */ 'dump.heapsnapshot');
```

```
$ node --expose-gc app.js

$ v8-heap-analyzer --input dump.heapsnapshot --explore
```


