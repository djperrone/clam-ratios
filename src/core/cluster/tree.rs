//! A `Tree` represents a hierarchy of "similar" instances from a metric-`Space`.

use distances::Number;

use crate::dataset::Dataset;

use super::{Cluster, PartitionCriteria};

/// A `Tree` represents a hierarchy of "similar" instances from a metric-`Space`.
///
/// Typically one will chain calls to `new`, `build`, and finally
/// `partition` to construct a fully realized `Tree`.
///
/// # Type Parameters
///
/// - `T`: The type of the instances in the `Tree`.
/// - `U`: The type of the distance values between instances.
/// - `D`: The type of the `Dataset` from which the `Tree` is built.
#[derive(Debug)]
pub struct Tree<T: Send + Sync + Copy, U: Number, D: Dataset<T, U>> {
    /// The dataset from which the tree is built.
    data: D,
    /// The root `Cluster` of the tree.
    root: Cluster<T, U>,
    /// The depth of the tree.
    depth: usize,
    /// The type of the instances in the tree.
    center: T,
}

impl<T: Send + Sync + Copy, U: Number, D: Dataset<T, U>> Tree<T, U, D> {
    /// Constructs a new `Tree` for a given dataset. Importantly,
    /// this does not build nor partition the tree.
    ///
    /// # Arguments
    /// dataset: The dataset from which the tree will be built
    pub fn new(data: D, seed: Option<u64>) -> Self {
        let root = Cluster::new_root(&data, data.indices(), seed);
        let depth = root.max_leaf_depth();
        let center = root.center;
        Self {
            data,
            root,
            depth,
            center,
        }
    }

    /// Returns a reference to dataset associated with the tree
    pub const fn data(&self) -> &D {
        &self.data
    }

    /// # Returns
    /// A reference to the root `Cluster` of the tree
    pub(crate) const fn root(&self) -> &Cluster<T, U> {
        &self.root
    }

    /// # Returns
    /// A reference to the root `Cluster` of the tree
    pub(crate) const fn depth(&self) -> usize {
        self.depth
    }

    /// # Returns
    /// A reference to the root `Cluster` of the tree
    pub(crate) const fn center(&self) -> T {
        self.center
    }

    /// # Returns
    /// The cardinality of the `Tree`
    pub const fn cardinality(&self) -> usize {
        self.root.cardinality
    }

    /// # Returns
    /// The radius of the `Tree`
    pub const fn radius(&self) -> U {
        self.root.radius
    }

    /// Partitions the `Tree` based off of a given criteria
    ///
    /// # Arguments
    /// criteria: A `PartitionCriteria` through which the `Tree`'s root will be partitioned.
    ///
    /// # Returns
    /// A new `Tree` with a partitioned root.
    #[must_use]
    pub fn partition(mut self, criteria: &PartitionCriteria<T, U>) -> Self {
        self.root = self.root.partition(&mut self.data, criteria);
        self
    }

    /// Returns the indices contained in the root of the `Tree`.
    pub fn indices(&self) -> &[usize] {
        self.data.indices()
    }
}
