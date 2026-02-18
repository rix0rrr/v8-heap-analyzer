use crate::types::NodeId;
use std::collections::HashMap;

/// Lengauer-Tarjan dominator tree algorithm
///
/// This implements the classic algorithm from "A Fast Algorithm for Finding Dominators
/// in a Flowgraph" (Lengauer & Tarjan, 1979). It computes the immediate dominator (idom)
/// for each node in a directed graph.
///
/// A node d dominates a node n if every path from the root to n must go through d.
/// The immediate dominator of n is the unique node that strictly dominates n but does
/// not strictly dominate any other node that strictly dominates n.
///
/// Time complexity: O(E * α(V)) where α is the inverse Ackermann function (nearly linear)
///
/// # Arguments
/// * `graph` - The graph to analyze (must implement GraphOps trait)
/// * `roots` - The root nodes to start the dominator analysis from
///
/// # Returns
/// A HashMap mapping each node to its immediate dominator
pub fn lengauer_tarjan<'a, G>(graph: &'a G, roots: &[NodeId]) -> HashMap<NodeId, NodeId>
where
    G: GraphOps<'a>,
{
    let mut lt = LengauerTarjan::new(graph.node_count());

    // Run DFS from all roots
    for &root in roots {
        lt.dfs(graph, root, NodeId::MAX);
    }

    lt.compute_dominators(graph)
}

/// Trait for graph operations required by the Lengauer-Tarjan algorithm
pub trait GraphOps<'a> {
    /// Returns the total number of nodes in the graph
    fn node_count(&self) -> usize;

    type PredIter: Iterator<Item = NodeId> + 'a;
    type SuccIter: Iterator<Item = NodeId> + 'a;

    /// Returns all predecessors (incoming edges) for a given node
    fn predecessors(&'a self, node: NodeId) -> Self::PredIter;

    /// Returns all successors (outgoing edges) for a given node
    fn successors(&'a self, node: NodeId) -> Self::SuccIter;
}

struct LengauerTarjan {
    // DFS state
    dfnum: Vec<NodeId>,
    vertex: Vec<NodeId>,
    parent: Vec<NodeId>,
    n: NodeId,

    // Dominator computation
    semi: Vec<NodeId>,
    ancestor: Vec<NodeId>,
    best: Vec<NodeId>,
    idom: Vec<NodeId>,
    samedom: Vec<NodeId>,
    bucket: Vec<Vec<NodeId>>,
}

impl LengauerTarjan {
    fn new(node_count: usize) -> Self {
        Self {
            dfnum: vec![NodeId::MAX; node_count],
            vertex: vec![NodeId::MAX; node_count],
            parent: vec![NodeId::MAX; node_count],
            n: 0,
            semi: vec![NodeId::MAX; node_count],
            ancestor: vec![NodeId::MAX; node_count],
            best: vec![NodeId::MAX; node_count],
            idom: vec![NodeId::MAX; node_count],
            samedom: vec![NodeId::MAX; node_count],
            bucket: vec![Vec::new(); node_count],
        }
    }

    fn dfs<'a, G: GraphOps<'a>>(&mut self, graph: &'a G, node: NodeId, p: NodeId) {
        if self.dfnum[node as usize] != NodeId::MAX {
            return;
        }

        self.dfnum[node as usize] = self.n;
        self.vertex[self.n as usize] = node;
        self.parent[node as usize] = p;
        self.n += 1;

        for succ in graph.successors(node) {
            self.dfs(graph, succ, node);
        }
    }

    fn compute_dominators<'a, G: GraphOps<'a>>(mut self, graph: &'a G) -> HashMap<NodeId, NodeId> {
        // Process nodes in reverse DFS order
        for i in (1..self.n).rev() {
            let w = self.vertex[i as usize];
            let p = self.parent[w as usize];

            // Skip if parent is invalid (shouldn't happen for i >= 1)
            if p == NodeId::MAX {
                continue;
            }

            let mut s = p;

            // Compute semidominator
            for v in graph.predecessors(w) {
                // Skip predecessors not visited in DFS
                if self.dfnum[v as usize] == NodeId::MAX {
                    continue;
                }

                let s_prime = if self.dfnum[v as usize] <= self.dfnum[w as usize] {
                    v
                } else {
                    let ancestor_result = self.ancestor_with_lowest_semi(v);
                    self.semi[ancestor_result as usize]
                };

                if self.dfnum[s_prime as usize] < self.dfnum[s as usize] {
                    s = s_prime;
                }
            }

            self.semi[w as usize] = s;
            self.bucket[s as usize].push(w);
            self.link(p, w);

            // Process bucket
            let bucket_items: Vec<_> = self.bucket[p as usize].drain(..).collect();
            for v in bucket_items {
                let y = self.ancestor_with_lowest_semi(v);
                if self.semi[y as usize] == self.semi[v as usize] {
                    self.idom[v as usize] = p;
                } else {
                    self.samedom[v as usize] = y;
                }
            }
        }

        // Adjust idom for nodes with samedom
        for i in 1..self.n {
            let w = self.vertex[i as usize];
            if self.samedom[w as usize] != NodeId::MAX {
                self.idom[w as usize] = self.idom[self.samedom[w as usize] as usize];
            }
        }

        // Build result map
        let mut result = HashMap::new();
        for i in 0..self.vertex.len() {
            let node = self.vertex[i];
            if node != NodeId::MAX && self.idom[node as usize] != NodeId::MAX {
                result.insert(node, self.idom[node as usize]);
            }
        }
        result
    }

    fn ancestor_with_lowest_semi(&mut self, v: NodeId) -> NodeId {
        let a = self.ancestor[v as usize];
        if a != NodeId::MAX && self.ancestor[a as usize] != NodeId::MAX {
            let b = self.ancestor_with_lowest_semi(a);
            self.ancestor[v as usize] = self.ancestor[a as usize];
            if self.dfnum[self.semi[b as usize] as usize]
                < self.dfnum[self.semi[self.best[v as usize] as usize] as usize]
            {
                self.best[v as usize] = b;
            }
        }
        self.best[v as usize]
    }

    fn link(&mut self, p: NodeId, n: NodeId) {
        self.ancestor[n as usize] = p;
        self.best[n as usize] = n;
    }
}

pub struct IterWrapper<'a> {
    iter: Box<dyn Iterator<Item = NodeId> + 'a>,
}
impl<'a> IterWrapper<'a> {
    pub fn new(iter: impl Iterator<Item = NodeId> + 'a) -> Self {
        IterWrapper {
            iter: Box::new(iter),
        }
    }
}

impl<'a> Iterator for IterWrapper<'a> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestGraph {
        preds: Vec<Vec<NodeId>>,
        succs: Vec<Vec<NodeId>>,
    }

    impl<'a> GraphOps<'a> for TestGraph {
        type PredIter = IterWrapper<'a>;
        type SuccIter = IterWrapper<'a>;

        fn node_count(&self) -> usize {
            self.preds.len()
        }

        fn predecessors(&'a self, node: NodeId) -> Self::PredIter {
            IterWrapper::new(self.preds[node as usize].iter().copied())
        }

        fn successors(&'a self, node: NodeId) -> Self::SuccIter {
            IterWrapper::new(self.succs[node as usize].iter().copied())
        }
    }

    #[test]
    fn test_simple_dominator_tree() {
        // Graph: 0 -> 1 -> 2
        //            \-> 3 -> 2
        // Node 0 dominates all, node 1 dominates 2
        let graph = TestGraph {
            preds: vec![
                vec![],     // 0: no predecessors (root)
                vec![0],    // 1: predecessor is 0
                vec![1, 3], // 2: predecessors are 1 and 3
                vec![1],    // 3: predecessor is 1
            ],
            succs: vec![
                vec![1],    // 0: successor is 1
                vec![2, 3], // 1: successors are 2 and 3
                vec![],     // 2: no successors
                vec![2],    // 3: successor is 2
            ],
        };

        let idom = lengauer_tarjan(&graph, &[0]);

        assert_eq!(idom.get(&1), Some(&0)); // 0 dominates 1
        assert_eq!(idom.get(&2), Some(&1)); // 1 dominates 2
        assert_eq!(idom.get(&3), Some(&1)); // 1 dominates 3
    }
}
