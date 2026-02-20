use std::iter::Copied;

use fixedbitset::FixedBitSet;
use petgraph::{
    Directed,
    visit::{GraphBase, GraphProp, IntoNeighbors, VisitMap, Visitable},
};

use crate::{graph::v8_heap_graph::V8HeapGraph, types::NodeId};

impl GraphProp for V8HeapGraph {
    #[doc = r" The kind of edges in the graph."]
    type EdgeType = Directed;
}

impl GraphBase for V8HeapGraph {
    #[doc = r" edge identifier"]
    type EdgeId = NodeId;

    #[doc = r" node identifier"]
    type NodeId = NodeId;
}

impl<'a> IntoNeighbors for &'a V8HeapGraph {
    type Neighbors = Copied<std::slice::Iter<'a, NodeId>>;

    #[doc = r" Return an iterator of the neighbors of node `a`."]
    fn neighbors(self, a: Self::NodeId) -> Self::Neighbors {
        self.out_neighbors(a).iter().copied()
    }
}

impl Visitable for V8HeapGraph {
    #[doc = r" The associated map type"]
    type Map = MyFixedBitSet;

    #[doc = r" Create a new visitor map"]
    fn visit_map(&self) -> Self::Map {
        MyFixedBitSet(FixedBitSet::with_capacity(self.total_node_count()))
    }

    #[doc = r" Reset the visitor map (and resize to new size of graph if needed)"]
    fn reset_map(&self, map: &mut Self::Map) {
        map.0.clear();
        map.0.grow(self.total_node_count());
    }
}

/// Newtype so we can implement VisitMap for FixedBitSet
pub struct MyFixedBitSet(FixedBitSet);

impl VisitMap<NodeId> for MyFixedBitSet {
    fn visit(&mut self, a: NodeId) -> bool {
        !self.0.put(a as usize)
    }

    fn is_visited(&self, a: &NodeId) -> bool {
        self.0.contains(*a as usize)
    }
}
