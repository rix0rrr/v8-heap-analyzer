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
    type Neighbors = NeighborsIter<'a>;

    #[doc = r" Return an iterator of the neighbors of node `a`."]
    fn neighbors(self, a: Self::NodeId) -> Self::Neighbors {
        let edges = self.edges(a);
        NeighborsIter {
            i: self.edge_info.to_node_field(),
            edge_stride: self.edge_info.stride(),
            edges,
        }
    }
}

pub struct NeighborsIter<'a> {
    edges: &'a [u32],
    i: usize,
    edge_stride: usize,
}

impl<'a> Iterator for NeighborsIter<'a> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.edges.len() {
            let ret = self.edges[self.i];
            self.i += self.edge_stride;
            Some(ret)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.edges.len() + self.edge_stride - self.i) / self.edge_stride;
        (remaining, Some(remaining))
    }
}

impl Visitable for V8HeapGraph {
    #[doc = r" The associated map type"]
    type Map = MyFixedBitSet;

    #[doc = r" Create a new visitor map"]
    fn visit_map(self: &Self) -> Self::Map {
        MyFixedBitSet(FixedBitSet::with_capacity(self.node_count()))
    }

    #[doc = r" Reset the visitor map (and resize to new size of graph if needed)"]
    fn reset_map(self: &Self, map: &mut Self::Map) {
        map.0.clear();
        map.0.grow(self.node_count());
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
