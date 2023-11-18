use crate::{Cluster, ClusterSet, Dataset, Edge, Instance};
use distances::Number;
use std::collections::HashSet;

/// Filler function to select clusters for graph
pub fn select_clusters<U: Number>(root: &Cluster<U>) -> ClusterSet<U> {
    let height = root.depth();
    let mut selected_clusters = ClusterSet::new();
    for c in root.subtree() {
        if c.depth() == height / 2 {
            selected_clusters.insert(c);
        }
    }
    selected_clusters
}

/// Detects edges between clusters
/// TODO! Add more documentation
/// TODO! Refactor for better performance
/// TODO! Generalize over different hashers?...
#[allow(clippy::implicit_hasher)]
pub fn detect_edges<'a, I: Instance, U: Number, D: Dataset<I, U>>(
    clusters: &ClusterSet<'a, U>,
    data: &D,
) -> HashSet<Edge<'a, U>> {
    let mut edges = HashSet::new();
    for (i, c1) in clusters.iter().enumerate() {
        for (j, c2) in clusters.iter().enumerate().skip(i + 1) {
            if i != j {
                let distance = c1.distance_to_other(data, c2);
                if distance <= c1.radius() + c2.radius() {
                    edges.insert(Edge::new(c1, c2, distance));
                }
            }
        }
    }

    edges
}
