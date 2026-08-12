//! The dependency graph (G8.2).
//!
//! Nodes are [`SymbolId`]s; edges are typed relationships. Traversal is always bounded by a
//! depth. Relationships that cannot be resolved precisely are recorded as
//! [`EdgeKind::Unknown`] rather than being invented.

use std::collections::{BTreeMap, VecDeque};

use crate::types::{DependencyEdge, EdgeKind, SymbolId};

/// A directed, typed graph over symbols.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    edges: Vec<DependencyEdge>,
    out: BTreeMap<SymbolId, Vec<(SymbolId, EdgeKind)>>,
    inc: BTreeMap<SymbolId, Vec<(SymbolId, EdgeKind)>>,
}

impl DependencyGraph {
    /// An empty graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a typed edge `from -> to`.
    pub fn add_edge(&mut self, from: SymbolId, to: SymbolId, kind: EdgeKind) {
        self.out
            .entry(from.clone())
            .or_default()
            .push((to.clone(), kind));
        self.inc
            .entry(to.clone())
            .or_default()
            .push((from.clone(), kind));
        self.edges.push(DependencyEdge { from, to, kind });
    }

    /// All edges.
    #[must_use]
    pub fn edges(&self) -> &[DependencyEdge] {
        &self.edges
    }

    /// Outgoing edges from a symbol.
    #[must_use]
    pub fn outgoing(&self, id: &SymbolId) -> &[(SymbolId, EdgeKind)] {
        self.out.get(id).map_or(&[], Vec::as_slice)
    }

    /// Incoming edges to a symbol.
    #[must_use]
    pub fn incoming(&self, id: &SymbolId) -> &[(SymbolId, EdgeKind)] {
        self.inc.get(id).map_or(&[], Vec::as_slice)
    }

    /// Distinct undirected neighbours of a symbol.
    #[must_use]
    pub fn neighbors(&self, id: &SymbolId) -> Vec<SymbolId> {
        let mut seen = std::collections::BTreeSet::new();
        for (to, _) in self.outgoing(id) {
            seen.insert(to.clone());
        }
        for (from, _) in self.incoming(id) {
            seen.insert(from.clone());
        }
        seen.into_iter().collect()
    }

    /// Breadth-first distances from a set of seeds, over undirected edges, bounded by
    /// `max_depth`. Determinism: seeds and the resulting map are ordered by `SymbolId`.
    #[must_use]
    pub fn bounded_distances(&self, seeds: &[SymbolId], max_depth: u32) -> BTreeMap<SymbolId, u32> {
        let mut dist: BTreeMap<SymbolId, u32> = BTreeMap::new();
        let mut queue: VecDeque<(SymbolId, u32)> = VecDeque::new();
        let mut ordered_seeds: Vec<SymbolId> = seeds.to_vec();
        ordered_seeds.sort();
        ordered_seeds.dedup();
        for s in ordered_seeds {
            dist.entry(s.clone()).or_insert(0);
            queue.push_back((s, 0));
        }
        while let Some((node, d)) = queue.pop_front() {
            if d >= max_depth {
                continue;
            }
            let mut nbrs = self.neighbors(&node);
            nbrs.sort();
            for n in nbrs {
                if !dist.contains_key(&n) {
                    dist.insert(n.clone(), d + 1);
                    queue.push_back((n, d + 1));
                }
            }
        }
        dist
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(s: &str) -> SymbolId {
        SymbolId(s.to_string())
    }

    #[test]
    fn traversal_is_bounded_and_deterministic() {
        let mut g = DependencyGraph::new();
        g.add_edge(sid("a"), sid("b"), EdgeKind::Calls);
        g.add_edge(sid("b"), sid("c"), EdgeKind::Calls);
        g.add_edge(sid("c"), sid("d"), EdgeKind::Calls);

        let d1 = g.bounded_distances(&[sid("a")], 1);
        assert_eq!(d1.get(&sid("a")), Some(&0));
        assert_eq!(d1.get(&sid("b")), Some(&1));
        assert_eq!(d1.get(&sid("c")), None, "depth-bounded");

        let d2 = g.bounded_distances(&[sid("a")], 2);
        assert_eq!(d2.get(&sid("c")), Some(&2));
        assert_eq!(d2.get(&sid("d")), None);

        // determinism
        assert_eq!(
            g.bounded_distances(&[sid("a")], 3),
            g.bounded_distances(&[sid("a")], 3)
        );
    }

    #[test]
    fn incoming_outgoing() {
        let mut g = DependencyGraph::new();
        g.add_edge(sid("caller"), sid("callee"), EdgeKind::Calls);
        assert_eq!(g.outgoing(&sid("caller")).len(), 1);
        assert_eq!(g.incoming(&sid("callee")).len(), 1);
        assert_eq!(g.incoming(&sid("caller")).len(), 0);
    }
}
