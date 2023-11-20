use std::cmp::Ordering;
use crate::{Cluster, ClusterSet, Dataset, Edge, Instance};
use distances::Number;
use std::collections::{BinaryHeap, HashSet};
use crate::chaoda::pretrained_models;
use crate::core::cluster::Ratios;

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


pub struct ClusterWrapper<'a, U: Number> {
    pub cluster: &'a Cluster<U>,
    pub score: f64
}

impl<'a, U: Number> PartialEq for ClusterWrapper<'a, U> {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl<'a, U: Number> Eq for ClusterWrapper<'a, U> {}

impl<'a, U: Number> Ord for ClusterWrapper<'a, U> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.partial_cmp(&other.score).unwrap_or(Ordering::Equal)
    }
}

impl<'a, U: Number> PartialOrd for ClusterWrapper<'a, U> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn avg_score(ratio : Ratios) -> f64 {
    let mut score: f64 = 0.0;
    let mut count: f64 = 0.0;
    let scorers = pretrained_models::get_meta_ml_scorers();
    for model in scorers {
        score += model.1(ratio);
        count += 1.0;
    }
    return score / count;
}


pub fn score_clusters<'a, U: Number>(root: &'a Cluster<U>, scoring_function: fn(Ratios) -> f64) -> BinaryHeap<ClusterWrapper<'a, U>>{
    let mut clusters = root.subtree();
    let mut scored_clusters: BinaryHeap<ClusterWrapper<'a, U>> = BinaryHeap::new();

    for cluster in clusters {
        let cluster_score = cluster.ratios().map_or(0.0, |value| scoring_function(value));
        scored_clusters.push(ClusterWrapper{cluster: &cluster, score: cluster_score })
    }

    return scored_clusters;
}

pub fn get_clusterset<'a, U: Number>(clusters: BinaryHeap<ClusterWrapper<'a, U>>) -> ClusterSet<'a, U>{
    let mut cluster_set : HashSet<&'a Cluster<U>> = HashSet::new();
    let mut clusters : BinaryHeap<&ClusterWrapper<'a, U>> = BinaryHeap::from(clusters.iter().clone().collect::<Vec<_>>());

    while clusters.len() > 0 {
        let best = clusters.pop().unwrap().cluster;
        clusters = clusters.into_iter().filter(|item| {
            !item.cluster.is_ancestor_of(best) && !item.cluster.is_descendant_of(best)
        }).collect();
        cluster_set.insert(best);
    }

    return cluster_set;
}
