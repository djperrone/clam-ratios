//! Criteria used for partitioning `Cluster`s.

use super::cluster::Cluster;
use super::dataset::Dataset;
use super::number::Number;

pub trait PartitionCriterion<T: Number, D: Dataset<T, f64>>: std::fmt::Debug + Send + Sync {
    fn check(&self, c: &Cluster<T, D>) -> bool;
}

#[derive(Debug)]
pub struct PartitionCriteria<T: Number, D: Dataset<T, f64>> {
    criteria: Vec<Box<dyn PartitionCriterion<T, D>>>,
    check_all: bool,
}

impl<'a, T: Number, D: Dataset<T, f64>> PartitionCriteria<T, D> {
    pub fn new(check_all: bool) -> Self {
        Self {
            criteria: Vec::new(),
            check_all,
        }
    }

    pub fn with_max_depth(mut self, threshold: usize) -> Self {
        self.criteria.push(Box::new(MaxDepth(threshold)));
        self
    }

    pub fn with_min_cardinality(mut self, threshold: usize) -> Self {
        self.criteria.push(Box::new(MinCardinality(threshold)));
        self
    }

    pub fn with_custom(mut self, c: Box<dyn PartitionCriterion<T, D>>) -> Self {
        self.criteria.push(c);
        self
    }

    pub fn check(&self, cluster: &Cluster<'a, T, D>) -> bool {
        !cluster.is_singleton()
            && if self.check_all {
                self.criteria.iter().all(|c| c.check(cluster))
            } else {
                self.criteria.iter().any(|c| c.check(cluster))
            }
    }
}

#[derive(Debug, Clone)]
struct MaxDepth(usize);

impl<T: Number, D: Dataset<T, f64>> PartitionCriterion<T, D> for MaxDepth {
    fn check(&self, c: &Cluster<T, D>) -> bool {
        c.depth() < self.0
    }
}

#[derive(Debug, Clone)]
struct MinCardinality(usize);

impl<T: Number, D: Dataset<T, f64>> PartitionCriterion<T, D> for MinCardinality {
    fn check(&self, c: &Cluster<T, D>) -> bool {
        c.cardinality() > self.0
    }
}
