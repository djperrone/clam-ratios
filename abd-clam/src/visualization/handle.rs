use crate::builder::{detect_edges, select_clusters};
use crate::visualization::utils;
use crate::{Graph, PartitionCriteria, Tree, VecDataset};

struct Handle<'a> {
    tree: Tree<Vec<f32>, f32, VecDataset<Vec<f32>, f32, bool>>,
    graph: Graph<'a, f32>,
}

impl<'a> Handle<'a> {
    fn new(tree: Tree<Vec<f32>, f32, VecDataset<Vec<f32>, f32, bool>>) -> Self {
        let data = VecDataset::new(
            "test".to_string(),
            vec![
                vec![10.],
                vec![1.],
                vec![-5.],
                vec![8.],
                vec![3.],
                vec![2.],
                vec![0.5],
                vec![0.],
            ],
            utils::euclidean::<f32, f32>,
            false,
            None,
        );
        let partition_criteria = PartitionCriteria::default();

        let tree = Tree::new(data, Some(42)).partition(&partition_criteria);

        let clusters = select_clusters(tree.root());
        let edges = detect_edges(&clusters, tree.data());

        let graph = Graph::new(clusters, edges).unwrap();

        Self { tree, graph }
    }
}
