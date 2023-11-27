use abd_clam::{cluster_selection, Cakes, Dataset, PartitionCriteria, Tree, VecDataset};
use symagen::random_data;

use std::collections::{BinaryHeap, HashMap, HashSet};

use abd_clam::cluster_selection::{avg_score, get_clusterset, score_clusters, select_clusters_for_visualization};
use rand::{distributions::Bernoulli, prelude::Distribution, Rng};

mod utils;

fn metric(x: &Vec<f32>, y: &Vec<f32>) -> f32 {
    distances::vectors::euclidean(x, y)
}

fn euclidean(x: &Vec<f32>, y: &Vec<f32>) -> f32 {
    x.iter()
        .zip(y.iter())
        .map(|(a, b)| a - b)
        .map(|v| v * v)
        .sum::<f32>()
        .sqrt()
}

#[test]
fn scoring() {
    let data = utils::gen_dataset(1000, 10, 42, utils::euclidean);
    let metric = data.metric();

    let partition_criteria: PartitionCriteria<f32> = PartitionCriteria::default();
    let tree = Tree::new(data, Some(42)).partition(&partition_criteria);
    let root = tree.root();
    let mut priority_queue = cluster_selection::score_clusters(&root, Box::new(avg_score));

    assert_eq!(priority_queue.len(), root.subtree().len());

    let mut prev_value: f64;
    let mut curr_value: f64;

    prev_value = priority_queue.pop().unwrap().score;
    while !priority_queue.is_empty() {
        curr_value = priority_queue.pop().unwrap().score;
        assert!(prev_value >= curr_value && curr_value >= 0.0);
        prev_value = curr_value;
    }

    let priority_queue = score_clusters(&root, Box::new(avg_score));
    let cluster_set = get_clusterset(priority_queue);
    // let cluster_set = select_clusters_for_visualization(tree.root(), Some(String::from("dt_euclidean_cc")));
    assert!(!cluster_set.contains(tree.root()));
    assert!(cluster_set.len() > 1);
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
