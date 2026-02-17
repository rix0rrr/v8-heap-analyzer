use super::super::{snapshot::SnapshotFile, types::NodeId};

// TODO: Perhaps we can make this an araay of structures for better cache locality

#[derive(Debug)]
pub struct V8HeapGraph {
    node_count: usize,
    nodes: Vec<NodeId>,
    edges: Edges,
    strings: Vec<String>,
    pub node_fields: NodeFields,
    pub edge_fields: EdgeFields,

    /// For every node, where in the "edges" array its edges start
    node_out_edges: Vec<NodeId>,
    node_in_edges: Vec<Vec<NodeId>>,
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

impl V8HeapGraph {
    pub fn nodes(&self) -> impl Iterator<Item = NodeId> {
        (0 as NodeId)..(self.node_count() as NodeId)
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
    fn from(value: SnapshotFile) -> Self {
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

        V8HeapGraph {
            node_count,
            nodes: value.nodes,
            edges,
            strings: value.strings,
            node_out_edges,
            node_in_edges,
            node_fields,
            edge_fields,
        }
    }
}

// For now these have static knowledge of all fields, but they validate
// against the actual fields we're seeing.
#[derive(Debug)]
pub struct NodeFields {
    stride: usize,
    trace_node_id: Option<usize>,
    detachedness: Option<usize>,
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
            detachedness: fields.iter().position(|x| x == "detachedness"),
        }
    }

    pub fn edge_count(&self, nodes: &[NodeId], i: NodeId) -> NodeId {
        nodes[i as usize * self.stride() + 4]
    }

    pub fn edge_count_field(&self) -> usize {
        4
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
