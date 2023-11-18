use abd_clam::{Cakes, Cluster, Dataset, EdgeSet, Graph, PartitionCriteria, Tree};
use std::collections::HashSet;
use std::fmt::Debug;

use abd_clam::builder::{detect_edges, select_clusters};
use abd_clam::ClusterSet;

mod utils;

#[test]
fn create_graph() {
    let data = utils::gen_dataset(1000, 10, 42, utils::euclidean);
    let metric = data.metric();

    let partition_criteria: PartitionCriteria<f32> = PartitionCriteria::default();
    let raw_tree = Tree::new(data, Some(42)).partition(&partition_criteria);

    let selected_clusters = select_clusters(raw_tree.root());

    let edges = detect_edges(&selected_clusters, raw_tree.data());

    let mut edges_ref = EdgeSet::new();
    for edge in edges.iter() {
        edges_ref.insert(edge);
    }

    let graph = Graph::new(selected_clusters.clone(), edges_ref);

    if let Ok(graph) = graph {
        assert_eq!(graph.clusters().len(), selected_clusters.len());
        assert_eq!(graph.edges().len(), edges.len());

        // graph.traverse()
    }
}
