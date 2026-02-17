// Make this object big so it's easy to find
globalThis.someObject = {
  foo: 'foo'.repeat(1_000_000).toUpperCase(),
  bar: 3,
};