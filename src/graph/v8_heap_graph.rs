use std::borrow::Cow;

use crate::{
    snapshot::StringOrStrings,
    utils::{escape_string, escape_string_chars, print_safe},
};

use super::super::{snapshot::SnapshotFile, types::NodeId};

// TODO: Perhaps we can make this an araay of structures for better cache locality

#[derive(Debug)]
pub struct V8HeapGraph {
    node_count: usize,
    nodes: Vec<NodeId>,
    edges: Edges,
    strings: Vec<String>,
    pub node_types: Vec<String>,
    pub edge_types: Vec<String>,
    pub node_fields: NodeFields,
    pub edge_fields: EdgeFields,

    /// For every node, where in the "edges" array its edges start
    node_out_edges: Vec<NodeId>,
    node_in_edges: Vec<Vec<NodeId>>,
}

impl V8HeapGraph {
    pub fn mem_size(&self) -> usize {
        let mut ret = 0;
        ret += self.nodes.len() * size_of::<NodeId>();
        ret += self.edges.mem_size();
        ret += self.node_out_edges.len() * size_of::<NodeId>();
        ret += self.node_in_edges.iter().fold(0, |acc, x| {
            acc + size_of::<Vec<NodeId>>() + x.len() * size_of::<NodeId>()
        });

        ret
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = NodeId> {
        (0 as NodeId)..(self.node_count() as NodeId)
    }

    pub fn iter_edges(&self) -> impl Iterator<Item = NodeId> {
        (0 as NodeId)..(self.edge_count() as NodeId)
    }

    pub fn node_range(&self, node: NodeId) -> &[NodeId] {
        let start = node as usize * self.node_fields.stride();
        &self.nodes[start..start + self.node_fields.stride()]
    }

    pub fn edge(&self, nr: NodeId) -> Edge<'_> {
        // Returns the index of the first element > nr
        //
        //   ┌────────────────────────┐    ┌─┬────▶┌───────────────┐
        //   │    Node 0 (0 edges)  0 │────┘ │     │    Edge 0     │
        //   ├────────────────────────┤      │ ┌──▶├───────────────┤
        //   │    Node 1 (1 edge)   0 │──────┘ │   │    Edge 1     │
        //   ├────────────────────────┤        │   ├───────────────┤
        //   │    Node 2 (2 edges)  1 │────────┘   │    Edge 2     │
        //   └────────────────────────┘            └───────────────┘
        //
        // For example, if we're looking for the Node for edge 0, we will
        // find Node 2 as a partition point (its value is 1, which is the first
        // one higher than the node we're looking for). Then subtract 1 to find
        // the actual source node, Node 1.
        let part = self.node_out_edges.partition_point(|probe| *probe <= nr);

        Edge {
            from_node: (part - 1) as NodeId,
            edges: &self.edges,
            edge: nr,
            strings: &self.strings,
        }
    }

    pub fn node(&self, id: NodeId) -> Node<'_> {
        Node {
            id,
            graph: self,
            node_range: self.node_range(id),
        }
    }

    pub fn string(&self, index: NodeId) -> &str {
        &self.strings[index as usize]
    }

    pub fn node_count(&self) -> usize {
        self.node_count
    }

    pub fn edge_count(&self) -> usize {
        self.edges.size()
    }

    /// Edge count for a node
    pub fn edge_count_for(&self, n: NodeId) -> NodeId {
        self.nodes[n as usize * self.node_fields.stride() + self.node_fields.edge_count_field()]
    }

    pub fn find_edge(&self, n: NodeId, edge_type: EdgeType, name: &str) -> Option<NodeId> {
        for edge in self.edges_for(n) {
            if edge.typ() == edge_type && edge.name_or_index().is_str(name) {
                return Some(edge.to_node());
            }
        }
        None
    }

    pub fn edges_for(&self, n: NodeId) -> impl Iterator<Item = Edge<'_>> {
        let start = self.node_out_edges[n as usize] as usize;
        let end = start + self.edge_count_for(n) as usize;

        (start..end).map(|e| self.edge(e as NodeId))
    }

    pub fn self_size_for(&self, n: NodeId) -> usize {
        self.nodes[n as usize * self.node_fields.stride() + self.node_fields.self_size_field()]
            as usize
    }

    /// All out edges for a Node
    pub fn out_edges(&self, node: NodeId) -> &[NodeId] {
        let start = self.node_out_edges[node as usize];
        self.edges
            .to(start as usize, self.edge_count_for(node) as usize)
    }

    /// All in edges for a Node
    pub fn in_edges(&self, node: NodeId) -> &[NodeId] {
        &self.node_in_edges[node as usize]
    }
}

impl From<SnapshotFile> for V8HeapGraph {
    fn from(mut value: SnapshotFile) -> Self {
        let node_count = value.snapshot.node_count;
        let node_fields = NodeFields::new(value.snapshot.meta.node_fields);
        let edge_fields = EdgeFields::new(value.snapshot.meta.edge_fields);

        let edges = Edges::new(
            value.edges,
            value.snapshot.edge_count,
            node_fields.stride() as NodeId,
        );

        let mut node_out_edges = Vec::<NodeId>::with_capacity(node_count);
        let mut node_in_edges = vec![Vec::new(); node_count];

        let edge_counts = value
            .nodes
            .iter()
            .skip(node_fields.edge_count_field())
            .step_by(node_fields.stride())
            .copied();

        let mut out_edge_index: NodeId = 0;
        let mut edge_idx: usize = 0;
        for (from_node, edge_count) in edge_counts.enumerate() {
            node_out_edges.push(out_edge_index);
            out_edge_index += edge_count;

            for _ in 0..edge_count {
                let to_node = edges.to1(edge_idx);
                node_in_edges[to_node as usize].push(from_node as NodeId);
                edge_idx += 1;
            }
        }

        let StringOrStrings::Strs(node_types) =
            std::mem::take(&mut value.snapshot.meta.node_types[0])
        else {
            panic!("Expected 'node_types[0]' to be a list of strings");
        };
        let StringOrStrings::Strs(edge_types) =
            std::mem::take(&mut value.snapshot.meta.edge_types[0])
        else {
            panic!("Expected 'edge_types[0]' to be a list of strings");
        };

        V8HeapGraph {
            node_count,
            nodes: value.nodes,
            edges,
            strings: value.strings,
            node_types,
            edge_types,
            node_out_edges,
            node_in_edges,
            node_fields,
            edge_fields,
        }
    }
}

pub struct Node<'a> {
    pub id: NodeId,
    node_range: &'a [NodeId],
    pub graph: &'a V8HeapGraph,
}

impl<'a> Node<'a> {
    pub fn typ(&self) -> NodeType {
        self.node_range[self.graph.node_fields.type_field()].into()
    }

    pub fn typ_str(&self) -> &'a str {
        let typ_id = self.node_range[self.graph.node_fields.type_field()];
        &self.graph.node_types[typ_id as usize]
    }

    pub fn name(&self) -> &'a str {
        let name_id = self.node_range[self.graph.node_fields.name_field()];
        self.graph.string(name_id)
    }

    pub fn print_safe_name(&self, max_len: usize) -> Cow<'a, str> {
        match self.typ() {
            NodeType::String => Cow::Owned(print_safe(self.name(), max_len)),
            _ => Cow::Borrowed(&self.name()),
        }
    }

    pub fn stable_id(&self) -> NodeId {
        self.node_range[self.graph.node_fields.stable_id()]
    }

    pub fn self_size(&self) -> usize {
        self.node_range[self.graph.node_fields.self_size_field()] as usize
    }

    pub fn edge_count(&self) -> usize {
        self.node_range[self.graph.node_fields.edge_count_field()] as usize
    }

    pub fn detachedness(&self) -> bool {
        self.node_range[self.graph.node_fields.detachedness_field()] == 1
    }
}

pub struct Edge<'a> {
    pub from_node: NodeId,
    edge: NodeId,
    strings: &'a Vec<String>,
    edges: &'a Edges,
}

impl<'a> Edge<'a> {
    pub fn typ(&self) -> EdgeType {
        self.edges.types[self.edge as usize].into()
    }

    pub fn typ_str(&self) -> &str {
        self.typ().as_str()
    }

    pub fn index(&self) -> NodeId {
        self.edges.names[self.edge as usize]
    }

    pub fn name_or_index(&self) -> NameOrIndex<'a> {
        let ni = self.edges.names[self.edge as usize];
        match self.typ() {
            EdgeType::Element => NameOrIndex::Index(ni),
            _ => NameOrIndex::Name(&self.strings[ni as usize]),
        }
    }

    pub fn to_node(&self) -> NodeId {
        self.edges.to_nodes[self.edge as usize]
    }
}

#[derive(Debug)]
pub enum NameOrIndex<'a> {
    Name(&'a str),
    Index(NodeId),
    Unsure(NodeId, &'a str),
}

impl<'a> NameOrIndex<'a> {
    pub fn is_str(&self, x: &str) -> bool {
        match self {
            NameOrIndex::Name(n) => *n == x,
            _ => false,
        }
    }
}

impl<'a> std::fmt::Display for NameOrIndex<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameOrIndex::Name(name) => write!(f, "{}", name),
            NameOrIndex::Index(n) => write!(f, "{}", n),
            NameOrIndex::Unsure(n, name) => write!(f, "{}/{}", n, name),
        }
    }
}

/// Edges in SoA format
#[derive(Debug)]
struct Edges {
    types: Vec<NodeId>,
    names: Vec<NodeId>,
    to_nodes: Vec<NodeId>,
}

impl Edges {
    pub fn new(snapshot_edges: Vec<NodeId>, edge_count: usize, node_stride: NodeId) -> Self {
        let mut ret = Edges {
            types: Vec::with_capacity(edge_count),
            names: Vec::with_capacity(edge_count),
            to_nodes: Vec::with_capacity(edge_count),
        };

        // All indirection for indexes out the window here :D
        for chunk in snapshot_edges.chunks_exact(3) {
            ret.types.push(chunk[0]);
            ret.names.push(chunk[1]);

            // The `to_node` fields in the input edges array are *indexes* into the `nodes`
            // array, not node identifiers. Divide them all by the node stride so we don't
            // have to do that later.
            ret.to_nodes.push(chunk[2] / node_stride);
        }

        ret
    }

    pub fn mem_size(&self) -> usize {
        (self.types.len() * size_of::<NodeId>())
            + (self.names.len() * size_of::<NodeId>())
            + (self.to_nodes.len() * size_of::<NodeId>())
    }

    pub fn to1(&self, edge: usize) -> NodeId {
        self.to_nodes[edge]
    }

    pub fn to(&self, start: usize, len: usize) -> &[NodeId] {
        &self.to_nodes[start..start + len]
    }

    pub fn size(&self) -> usize {
        self.types.len()
    }
}

// For now these have static knowledge of all fields, but they validate
// against the actual fields we're seeing.
#[derive(Debug)]
pub struct NodeFields {
    stride: usize,
    trace_node_id: Option<usize>,
    detachedness: usize,
}

impl NodeFields {
    pub fn new(fields: Vec<String>) -> Self {
        assert!(fields.len() >= 5);
        assert!(fields[0] == "type");
        assert!(fields[1] == "name");
        assert!(fields[2] == "id");
        assert!(fields[3] == "self_size");
        assert!(fields[4] == "edge_count");

        Self {
            stride: fields.len(),
            trace_node_id: fields.iter().position(|x| x == "trace_node_id"),
            detachedness: fields
                .iter()
                .position(|x| x == "detachedness")
                .expect("Did not find detachedness"),
        }
    }

    pub fn edge_count(&self, nodes: &[NodeId], i: NodeId) -> NodeId {
        nodes[i as usize * self.stride() + 4]
    }

    pub fn type_field(&self) -> usize {
        0
    }

    pub fn name_field(&self) -> usize {
        1
    }

    pub fn stable_id(&self) -> usize {
        2
    }

    pub fn self_size_field(&self) -> usize {
        3
    }

    pub fn edge_count_field(&self) -> usize {
        4
    }

    pub fn detachedness_field(&self) -> usize {
        self.detachedness
    }

    pub fn stride(&self) -> usize {
        self.stride
    }
}

// For now these have static knowledge of all fields, but they validate
// against the actual fields we're seeing.
#[derive(Debug)]
pub struct EdgeFields {}

impl EdgeFields {
    pub fn new(fields: Vec<String>) -> Self {
        assert!(fields.len() == 3);
        assert!(fields[0] == "type");
        assert!(fields[1] == "name_or_index");
        assert!(fields[2] == "to_node");
        Self {}
    }

    pub fn to_node_field(&self) -> usize {
        2
    }

    pub fn stride(&self) -> usize {
        3
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum NodeType {
    Hidden = 0,
    Array = 1,
    String = 2,
    Object = 3,
    Code = 4,
    Closure = 5,
    RegExp = 6,
    Number = 7,
    Native = 8,
    Synthetic = 9,
    ConcatString = 10,
    SlicedString = 11,
    Symbol = 12,
    BigInt = 13,
    ObjectShape = 14,
}

impl From<NodeId> for NodeType {
    fn from(value: NodeId) -> Self {
        match value {
            0 => NodeType::Hidden,
            1 => NodeType::Array,
            2 => NodeType::String,
            3 => NodeType::Object,
            4 => NodeType::Code,
            5 => NodeType::Closure,
            6 => NodeType::RegExp,
            7 => NodeType::Number,
            8 => NodeType::Native,
            9 => NodeType::Synthetic,
            10 => NodeType::ConcatString,
            11 => NodeType::SlicedString,
            12 => NodeType::Symbol,
            13 => NodeType::BigInt,
            14 => NodeType::ObjectShape,
            _ => panic!("Unexpecte node type"),
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum EdgeType {
    Context = 0,
    Element = 1,
    Property = 2,
    Internal = 3,
    Hidden = 4,
    Shortcut = 5,
    Weak = 6,
}

impl EdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeType::Context => "context",
            EdgeType::Element => "element",
            EdgeType::Property => "property",
            EdgeType::Internal => "internal",
            EdgeType::Hidden => "hidden",
            EdgeType::Shortcut => "shortcut",
            EdgeType::Weak => "weak",
        }
    }
}

impl From<NodeId> for EdgeType {
    fn from(value: NodeId) -> Self {
        match value {
            0 => EdgeType::Context,
            1 => EdgeType::Element,
            2 => EdgeType::Property,
            3 => EdgeType::Internal,
            4 => EdgeType::Hidden,
            5 => EdgeType::Shortcut,
            6 => EdgeType::Weak,
            _ => panic!("Unexpecte edge type"),
        }
    }
}
