use super::super::{snapshot::SnapshotFile, types::NodeId};

// TODO: Perhaps we can make this an araay of structures for better cache locality

#[derive(Debug)]
pub struct V8HeapGraph {
    node_count: usize,
    edge_count: usize,
    nodes: Vec<NodeId>,
    edges: Vec<NodeId>,
    strings: Vec<String>,
    /// For every node, where in the "edges" array its edges start
    node_edges: Vec<usize>,
    pub node_info: NodeFields,
    pub edge_info: EdgeFields,
}

impl V8HeapGraph {
    pub fn node_count(&self) -> usize {
        self.node_count
    }

    pub fn edge_count(&self) -> usize {
        self.edge_count
    }

    /// Edge count for a node
    pub fn edge_count_for(&self, n: NodeId) -> NodeId {
        self.nodes[n as usize * self.node_info.stride() + self.node_info.edge_count_field()]
    }

    /// All edges for a Node
    pub fn edges(&self, node: NodeId) -> &[NodeId] {
        let start = self.node_edges[node as usize];
        let end = start + self.edge_count_for(node) as usize * self.edge_info.stride();
        &self.edges[start..end]
    }
}

impl From<SnapshotFile> for V8HeapGraph {
    fn from(mut value: SnapshotFile) -> Self {
        let node_count = value.snapshot.node_count;

        let node_info = NodeFields::new(value.snapshot.meta.node_fields);
        let edge_info = EdgeFields::new(value.snapshot.meta.edge_fields);

        // Find starting indexes into `edges` array for every node
        let mut node_edges = Vec::<usize>::with_capacity(value.nodes.len());
        let mut i = node_info.edge_count_field();
        let mut start: usize = 0;
        for _ in 0..node_count {
            node_edges.push(start * edge_info.stride());
            start += value.nodes[i] as usize;
            i += node_info.stride();
        }

        // The `to_node` fields in the input edges array are *indexes* into the `nodes`
        // array, not node identifiers. Divide them all by the node stride so we don't
        // have to do that later.
        let node_stride = node_info.stride() as u32;
        for i in (edge_info.to_node_field()..value.edges.len()).step_by(edge_info.stride()) {
            value.edges[i] /= node_stride;
        }

        V8HeapGraph {
            node_count,
            edge_count: value.snapshot.edge_count,
            nodes: value.nodes,
            edges: value.edges,
            strings: value.strings,
            node_edges,
            node_info,
            edge_info,
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
