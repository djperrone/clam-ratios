use super::cluster::Cluster;
use super::dataset::Dataset;
use super::number::Number;

#[allow(dead_code)]
pub struct Manifold<'a, T: Number, D: Dataset<T, f64>> {
    root: Cluster<'a, T, D>,
}

// pub type CandidateNeighbors<T, U> = Vec<(Box<Cluster<T, U>>, U)>;
// candidate_neighbors: Option<CandidateNeighbors<T, U>>,

impl<'a, T: Number, D: Dataset<T, f64>> Manifold<'a, T, D> {}
