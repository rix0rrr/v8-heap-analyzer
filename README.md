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

What does this tool show me?
----------------------------

The most developed tool is the interactive explorer (`--explore`) and it will show you the **Dominator Tree** of the
object graph on the heap. The heap has a huge number of objects, and there are potentially many different objects with
references to any other given object, so the dominator tree is a way to make sense of why objects are alive. It
is helpful in answering the question: "what is holding on to

If an object D is a **dominator** of an object O, that means that D is the reason that O is alive. That is: if either
D disappears, or D drops its reference to O, then O will disappear.

The **dominator tree** is a tree structure where objects are shown under their immediate dominators.

In contrast to the Chrome Developer Tools tree, the sizes shown in the dominator tree are definitely those unique
attributable to a given object, and guaranteed not to be in objects with shared references.

The dominator tree isn't necessarily going to tell you exactly what fields of an object to cut, especially if the size
is in internals of the interpreter (like closure contexts). After you've used v8-heap-explorer to identify the most
likely culprits of your high memory usage, you might still want to open up Chrome DevTools to explore around the
indicated object types to figure out the exact culprit.
