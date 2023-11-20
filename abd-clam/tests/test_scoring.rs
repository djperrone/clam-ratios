use abd_clam::{Cakes, Cluster, Dataset, EdgeSet, Graph, PartitionCriteria, Tree};
use std::collections::HashSet;
use std::fmt::Debug;

use abd_clam::builder::{detect_edges, select_clusters};
use abd_clam::ClusterSet;
use symagen::random_data;

use abd_clam::builder::*;



mod utils;

#[test]
fn scoring(){
    let data = utils::gen_dataset(1000, 10, 42, utils::euclidean);
    let metric = data.metric();

    let partition_criteria: PartitionCriteria<f32> = PartitionCriteria::default();
    let raw_tree = Tree::new(data, Some(42)).partition(&partition_criteria).with_ratios(false);

    let mut root = raw_tree.root();

    let mut priority_queue = score_clusters(&root, avg_score);

    assert_eq!(priority_queue.len(), root.subtree().len());

    let mut prev_value: f64;
    let mut curr_value: f64;

    prev_value = priority_queue.pop().unwrap().score;
    while !priority_queue.is_empty() {
        curr_value = priority_queue.pop().unwrap().score;
        assert!(prev_value >= curr_value && curr_value >= 0.0);
        prev_value = curr_value;
    }

    let priority_queue = score_clusters(&root, avg_score);
    let cluster_set = get_clusterset(priority_queue);
    for i in &cluster_set {
        for j in &cluster_set {
            if i != j {
                assert!(!i.is_descendant_of(j) && !i.is_ancestor_of(j));
            }
        }
    }

    for i in &root.subtree() {
        let mut ancestor_of = false;
        let mut descendant_of = false;
        for j in &cluster_set {
            if i.is_ancestor_of(j) {
                ancestor_of = true;
            }
            if i.is_descendant_of(j) {
                descendant_of = true
            }
        }
        assert!(ancestor_of || descendant_of || cluster_set.contains(i))
    }
}