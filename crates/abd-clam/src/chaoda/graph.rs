//! A `Graph` is a collection of `OddBall`s.

use core::{cmp::Reverse, ops::Index};
use std::collections::{BinaryHeap, HashSet};

use distances::Number;
use ndarray::prelude::*;
use ordered_float::OrderedFloat;
use rayon::prelude::*;

use crate::{Dataset, Instance};

use super::OddBall;

/// A `Graph` is a collection of `OddBall`s.
///
/// Two `OddBall`s have an edge between them if they have any overlapping volume,
/// i.e. if the distance between their centers is no greater than the sum of their
/// radii.
#[derive(Clone)]
pub struct Graph<U: Number> {
    /// The collection of `Component`s in the `Graph`.
    components: Vec<Component<U>>,
    /// Cumulative populations of the `Component`s in the `Graph`.
    populations: Vec<usize>,
    /// A `HashSet` of the `OddBall`s' `offset` and `cardinality` in the `Graph`.
    members: HashSet<(usize, usize)>,
}

// , C: OddBall<U>, const N: usize
impl<U: Number> Graph<U> {
    /// Create a new `Graph` from a `Tree`.
    ///
    /// # Arguments
    ///
    /// * `tree`: The `Tree` to create the `Graph` from.
    /// * `cluster_scorer`: A function that scores `OddBall`s.
    /// * `min_depth`: The minimum depth at which to consider a `OddBall`.
    pub fn from_tree<I: Instance, D: Dataset<I, U>, C: OddBall<U>>(
        root: &C,
        data: &D,
        cluster_scorer: impl Fn(&[&C]) -> Vec<f32>,
        min_depth: usize,
    ) -> Self {
        let clusters = root.subtree();
        let scores = cluster_scorer(&clusters);

        // We use `OrderedFloat` to have the `Ord` trait implemented for `f64` so that we can use it in a `BinaryHeap`.
        // We use `Reverse` on `OddBall` so that we can bias towards selecting shallower `OddBall`s.
        // `OddBall`s are selected by highest score and then by shallowest depth.
        let mut candidates = clusters
            .into_iter()
            .zip(scores.into_iter().map(OrderedFloat))
            .filter(|(c, _)| c.is_leaf() || c.depth() >= min_depth)
            .map(|(c, s)| (s, Reverse(c)))
            .collect::<BinaryHeap<_>>();

        let mut clusters = vec![];
        while let Some((_, Reverse(c))) = candidates.pop() {
            clusters.push(c);
            // Remove `OddBall`s that are ancestors or descendants of the selected `OddBall`, so as not to have duplicates
            // in the `Graph`.
            candidates.retain(|&(_, Reverse(other))| !(c.is_ancestor_of(other) || c.is_descendant_of(other)));
        }
        clusters.sort_unstable_by_key(|c| c.offset());

        Self::from_clusters(&clusters, data)
    }

    /// Create a new `Graph` from a collection of `OddBall`s.
    pub fn from_clusters<I: Instance, D: Dataset<I, U>, C: OddBall<U>>(clusters: &[&C], data: &D) -> Self {
        let members = clusters.iter().map(|c| (c.offset(), c.cardinality())).collect();

        let c = Component::new(clusters, data);
        let [mut c, mut other] = c.partition();
        let mut components = vec![c];
        while !other.is_empty() {
            [c, other] = other.partition();
            components.push(c);
        }
        let populations = components
            .iter()
            .map(|c| c.population)
            .scan(0, |acc, x| {
                *acc += x;
                Some(*acc)
            })
            .collect::<Vec<_>>();
        Self {
            components,
            populations,
            members,
        }
    }

    /// Check whether the `Graph` contains a `OddBall` with the given `offset` and `cardinality`.
    #[must_use]
    pub fn contains(&self, offset: usize, cardinality: usize) -> bool {
        self.members.contains(&(offset, cardinality))
    }

    /// Iterate over the `OddBall`s in the `Graph`.
    pub fn iter_clusters(&self) -> impl Iterator<Item = &(usize, usize, usize)> {
        self.components.iter().flat_map(Component::iter_clusters)
    }

    /// Iterate over the lists of neighbors of the `OddBall`s in the `Graph`.
    pub fn iter_neighbors(&self) -> impl Iterator<Item = &[(usize, U)]> {
        self.components.iter().flat_map(Component::iter_neighbors)
    }

    /// Iterate over the anomaly properties of the `OddBall`s in the `Graph`.
    pub fn iter_anomaly_properties(&self) -> impl Iterator<Item = &Vec<f32>> {
        self.components.iter().flat_map(Component::iter_anomaly_properties)
    }

    /// Get the diameter of the `Graph`.
    pub fn diameter(&mut self) -> usize {
        self.components.iter_mut().map(Component::diameter).max().unwrap_or(0)
    }

    /// Get the neighborhood sizes of all `OddBall`s in the `Graph`.
    pub fn neighborhood_sizes(&mut self) -> Vec<&Vec<usize>> {
        self.components
            .iter_mut()
            .flat_map(Component::neighborhood_sizes)
            .collect()
    }

    /// Get the total number of points in the `Graph`.
    #[must_use]
    pub fn population(&self) -> usize {
        self.populations.last().copied().unwrap_or(0)
    }

    /// Iterate over the `Component`s in the `Graph`.
    pub(crate) fn iter_components(&self) -> impl Iterator<Item = &Component<U>> {
        self.components.iter()
    }

    /// Compute the stationary probability of each `OddBall` in the `Graph`.
    #[must_use]
    pub fn compute_stationary_probabilities(&self, num_steps: usize) -> Vec<f32> {
        self.components
            .par_iter()
            .flat_map(|c| c.compute_stationary_probabilities(num_steps))
            .collect()
    }

    /// Get the accumulated child-parent cardinality ratio of each `OddBall` in the `Graph`.
    #[must_use]
    pub fn accumulated_cp_car_ratios(&self) -> Vec<f32> {
        self.components
            .iter()
            .flat_map(Component::accumulated_cp_car_ratios)
            .copied()
            .collect()
    }

    /// Get the `Graph` as a single `Component` object.
    ///
    /// This will break the `Component` invariant that the `Component`s are
    /// connected subgraphs.
    #[must_use]
    pub fn as_single_component(&self) -> Self {
        if self.components.len() == 1 {
            // TODO: Ensure that the `Component` is in sorted order.
            return self.clone();
        }

        let (clusters, sort_indices) = {
            let mut clusters = self.iter_clusters().copied().enumerate().collect::<Vec<_>>();
            clusters.sort_unstable_by_key(|&(_, (o, _, _))| o);
            let sort_indices = clusters.iter().map(|&(i, _)| i).collect::<Vec<_>>();
            let clusters = clusters.into_iter().map(|(_, c)| c).collect();
            (clusters, sort_indices)
        };

        let adjacency_list = {
            let mut adjacency_list = self
                .iter_neighbors()
                .map(<[(usize, U)]>::to_vec)
                .zip(sort_indices.iter())
                .collect::<Vec<_>>();
            adjacency_list.sort_unstable_by_key(|&(_, i)| i);
            adjacency_list.into_iter().map(|(a, _)| a).collect()
        };

        let population = self.population();

        let accumulated_cp_car_ratios = {
            let mut accumulated_cp_car_ratios = self
                .accumulated_cp_car_ratios()
                .into_iter()
                .zip(sort_indices.iter())
                .collect::<Vec<_>>();
            accumulated_cp_car_ratios.sort_unstable_by_key(|&(_, i)| i);
            accumulated_cp_car_ratios.into_iter().map(|(r, _)| r).collect()
        };

        let anomaly_properties = {
            let mut anomaly_properties = self
                .iter_anomaly_properties()
                .map(Vec::clone)
                .zip(sort_indices.iter())
                .collect::<Vec<_>>();
            anomaly_properties.sort_unstable_by_key(|&(_, i)| i);
            anomaly_properties.into_iter().map(|(r, _)| r).collect()
        };

        let c = Component {
            clusters,
            adjacency_list,
            population,
            eccentricities: None,
            diameter: None,
            neighborhood_sizes: None,
            accumulated_cp_car_ratios,
            anomaly_properties,
        };

        Self {
            components: vec![c],
            populations: vec![population],
            members: self.members.clone(),
        }
    }

    /// Merge two `Graph`s from the save tree into a single `Graph`.
    ///
    /// `Cluster`s with larger depths are preferentially selected for the merged
    /// `Graph`. If two `Cluster`s in the merged `Graph` had an ancestor in
    /// either of the two input `Graph`s, then those two `Cluster`s will have an
    /// edge added between them.
    ///
    /// # Arguments
    ///
    /// * `other`: The other `Graph` to merge with.
    /// * `data`: The `Dataset` that the `Graph`s were created from.
    #[must_use]
    #[allow(unused_variables)]
    pub fn merge<I: Instance, D: Dataset<I, U>>(&self, other: &Self, data: &D) -> Self {
        let g1 = self.as_single_component();
        let g2 = other.as_single_component();

        todo!()
    }
}

/// A `Component` is a single connected subgraph of a `Graph`.
///
/// We break the `Graph` into connected `Component`s because this makes several
/// computations significantly easier to think about and implement.
#[derive(Clone)]
pub struct Component<U: Number> {
    /// The offsets, cardinalities, and indices of centers of the `OddBall`s in the `Component`.
    clusters: Vec<(usize, usize, usize)>,
    /// The adjacency list of the `Component`. Each `usize` is the index of a `OddBall`
    /// in the `clusters` field and the distance between the two `OddBall`s.
    adjacency_list: Vec<Vec<(usize, U)>>,
    /// The total number of points in the `OddBall`s in the `Component`.
    population: usize,
    /// Eccentricity of each `OddBall` in the `Component`.
    eccentricities: Option<Vec<usize>>,
    /// Diameter of the `Component`.
    diameter: Option<usize>,
    /// neighborhood sizes of each `OddBall` in the `Component` at each step through a BFT.
    neighborhood_sizes: Option<Vec<Vec<usize>>>,
    /// The accumulated child-parent cardinality ratio of each `OddBall` in the `Component`.
    accumulated_cp_car_ratios: Vec<f32>,
    /// The anomaly properties of the `OddBall`s in the `Component`.
    anomaly_properties: Vec<Vec<f32>>,
}

impl<U: Number> Component<U> {
    /// Create a new `Component` from a collection of `OddBall`s.
    fn new<I: Instance, D: Dataset<I, U>, C: OddBall<U>>(clusters: &[&C], data: &D) -> Self {
        // TODO: Replace this nested iteration with a more efficient algorithm using CAKES.
        let adjacency_list = clusters
            .par_iter()
            .enumerate()
            .map(|(i, c1)| {
                clusters
                    .par_iter()
                    .enumerate()
                    .filter(|&(j, _)| i != j)
                    .filter_map(|(j, c2)| {
                        let (r1, r2) = (c1.radius(), c2.radius());
                        let d = c1.distance_to_other(data, c2);
                        if d <= r1 + r2 {
                            Some((j, d))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .collect();

        let population = clusters.iter().map(|c| c.cardinality()).sum();
        let cluster_indices = clusters
            .iter()
            .map(|c| (c.offset(), c.cardinality(), c.arg_center()))
            .collect();
        let accumulated_cp_car_ratios = clusters.iter().map(|c| c.accumulated_cp_car_ratio()).collect();
        let anomaly_properties = clusters.iter().map(|c| c.ratios()).collect::<Vec<_>>();

        Self {
            clusters: cluster_indices,
            adjacency_list,
            population,
            eccentricities: None,
            diameter: None,
            neighborhood_sizes: None,
            accumulated_cp_car_ratios,
            anomaly_properties,
        }
    }

    /// Partition the `Component` into two `Component`s.
    ///
    /// The first component is a connected subgraph of the original `Component`
    /// and the second component is the rest of the original `Component`.
    ///
    /// This method is used when first constructing the `Graph` to find the
    /// connected subgraphs of the `Graph`.
    ///
    /// This method is meant to be used in a loop to find all connected subgraphs
    /// of a `Graph`. It resets the internal members of the `Component` that are
    /// computed lazily, i.e. the eccentricities, diameter, and neighborhood sizes.
    fn partition(mut self) -> [Self; 2] {
        // Perform a traversal of the adjacency list to find a connected subgraph.
        let mut visited = vec![false; self.clusters.len()];
        let mut stack = vec![0];
        while let Some(i) = stack.pop() {
            if visited[i] {
                continue;
            }
            visited[i] = true;
            for &(j, _) in &self.adjacency_list[i] {
                stack.push(j);
            }
        }
        let (p1, p2) = visited
            .iter()
            .zip(self.clusters.iter().copied())
            .zip(self.adjacency_list)
            .partition::<Vec<_>, _>(|((&v, _), _)| v);

        // Build a component from the clusters that were not visited in the traversal.
        let (clusters, mut adjacency_list): (Vec<_>, Vec<_>) = p2.into_iter().map(|((_, c), a)| (c, a)).unzip();
        // Remap indices in adjacency list
        for neighbors in &mut adjacency_list {
            for (j, _) in neighbors {
                let (old_offset, _, _) = self.clusters[*j];
                *j = clusters
                    .iter()
                    .position(|&(offset, _, _)| offset == old_offset)
                    .unwrap_or_else(|| unreachable!("OddBall not found in partitioned component"));
            }
        }
        let population = clusters.iter().map(|&(_, c, _)| c).sum();
        let accumulated_cp_car_ratios = self
            .accumulated_cp_car_ratios
            .iter()
            .zip(visited.iter())
            .filter_map(|(&r, &v)| if v { None } else { Some(r) })
            .collect();
        let anomaly_properties = self
            .anomaly_properties
            .iter()
            .zip(visited.iter())
            .filter_map(|(r, &v)| if v { None } else { Some(r.clone()) })
            .collect::<Vec<_>>();
        let other = Self {
            clusters,
            adjacency_list,
            population,
            eccentricities: None,
            diameter: None,
            neighborhood_sizes: None,
            accumulated_cp_car_ratios,
            anomaly_properties,
        };

        // Set the current component to the visited clusters.
        let (clusters, mut adjacency_list): (Vec<_>, Vec<_>) = p1.into_iter().map(|((_, c), a)| (c, a)).unzip();
        // Remap indices in adjacency list
        for neighbors in &mut adjacency_list {
            for (j, _) in neighbors {
                let (old_offset, _, _) = self.clusters[*j];
                *j = clusters
                    .iter()
                    .position(|&(offset, _, _)| offset == old_offset)
                    .unwrap_or_else(|| unreachable!("OddBall not found in partitioned component"));
            }
        }
        let population = clusters.iter().map(|&(_, c, _)| c).sum();
        let accumulated_cp_car_ratios = self
            .accumulated_cp_car_ratios
            .iter()
            .zip(visited.iter())
            .filter_map(|(&r, &v)| if v { Some(r) } else { None })
            .collect();
        let anomaly_properties = self
            .anomaly_properties
            .iter()
            .zip(visited.iter())
            .filter_map(|(r, &v)| if v { Some(r.clone()) } else { None })
            .collect::<Vec<_>>();

        self.clusters = clusters;
        self.adjacency_list = adjacency_list;
        self.population = population;
        self.eccentricities = None;
        self.diameter = None;
        self.neighborhood_sizes = None;
        self.accumulated_cp_car_ratios = accumulated_cp_car_ratios;
        self.anomaly_properties = anomaly_properties;

        [self, other]
    }

    /// Check if the `Component` has any `OddBall`s.
    fn is_empty(&self) -> bool {
        self.clusters.is_empty()
    }

    /// Iterate over the `OddBall`s in the `Component`.
    fn iter_clusters(&self) -> impl Iterator<Item = &(usize, usize, usize)> {
        self.clusters.iter()
    }

    /// Iterate over the lists of neighbors of the `OddBall`s in the `Component`.
    fn iter_neighbors(&self) -> impl Iterator<Item = &[(usize, U)]> {
        self.adjacency_list.iter().map(Vec::as_slice)
    }

    /// Iterate over the anomaly properties of the `OddBall`s in the `Component`.
    fn iter_anomaly_properties(&self) -> impl Iterator<Item = &Vec<f32>> {
        self.anomaly_properties.iter()
    }

    /// Get the number of `OddBall`s in the `Component`.
    pub fn cardinality(&self) -> usize {
        self.clusters.len()
    }

    /// Get the total number of points in the `Component`.
    pub const fn population(&self) -> usize {
        self.population
    }

    /// Get the diameter of the `Component`.
    pub fn diameter(&mut self) -> usize {
        if self.diameter.is_none() {
            if self.eccentricities.is_none() {
                self.compute_eccentricities();
            }
            let ecc = self
                .eccentricities
                .as_ref()
                .unwrap_or_else(|| unreachable!("We just computed the eccentricities"));
            self.diameter = Some(ecc.iter().copied().max().unwrap_or(0));
        }
        self.diameter
            .unwrap_or_else(|| unreachable!("We just computed the diameter"))
    }

    /// Compute the eccentricity of each `OddBall` in the `Component`.
    pub fn compute_eccentricities(&mut self) {
        self.eccentricities = Some(self.neighborhood_sizes().iter().map(Vec::len).collect());
    }

    /// Get the neighborhood sizes of all `OddBall`s in the `Component`.
    pub fn neighborhood_sizes(&mut self) -> &[Vec<usize>] {
        if self.neighborhood_sizes.is_none() {
            self.neighborhood_sizes = Some(
                (0..self.cardinality())
                    .into_par_iter()
                    .map(|i| self.compute_neighborhood_sizes(i))
                    .collect(),
            );
        }
        self.neighborhood_sizes
            .as_ref()
            .unwrap_or_else(|| unreachable!("We just computed the neighborhood sizes"))
    }

    /// Get the cumulative number of neighbors encountered after each step through a BFT.
    fn compute_neighborhood_sizes(&self, i: usize) -> Vec<usize> {
        let mut visited = vec![false; self.cardinality()];
        let mut neighborhood_sizes = Vec::new();
        let mut stack = vec![i];
        while let Some(i) = stack.pop() {
            if visited[i] {
                continue;
            }
            visited[i] = true;
            let new_neighbors = self.adjacency_list[i]
                .iter()
                .filter(|(j, _)| !visited[*j])
                .collect::<Vec<_>>();
            neighborhood_sizes.push(new_neighbors.len());
            stack.extend(new_neighbors.iter().map(|(j, _)| *j));
        }

        neighborhood_sizes
            .iter()
            .scan(0, |acc, x| {
                *acc += x;
                Some(*acc)
            })
            .collect()
    }

    /// Compute the stationary probability of each `OddBall` in the `Component`.
    pub fn compute_stationary_probabilities(&self, num_steps: usize) -> Vec<f32> {
        if self.cardinality() == 1 {
            // Singleton components need to be marked as anomalous.
            return vec![0.0];
        }

        let mut transition_matrix = vec![0_f32; self.cardinality() * self.cardinality()];
        for (i, neighbors) in self.adjacency_list.iter().enumerate() {
            for &(j, d) in neighbors {
                transition_matrix[i * self.cardinality() + j] = d.as_f32().recip();
            }
        }
        // Convert the transition matrix to an Array2
        let mut transition_matrix = Array2::from_shape_vec((self.cardinality(), self.cardinality()), transition_matrix)
            .unwrap_or_else(|e| unreachable!("We created a square Transition matrix: {e}"));

        // Normalize the transition matrix so that each row sums to 1
        for i in 0..self.cardinality() {
            let row_sum = transition_matrix.row(i).sum();
            transition_matrix.row_mut(i).mapv_inplace(|x| x / row_sum);
        }

        // Compute the stationary probabilities by squaring the transition matrix `num_steps` times
        for _ in 0..num_steps {
            transition_matrix = transition_matrix.dot(&transition_matrix);
        }

        // Compute the stationary probabilities by summing the rows of the transition matrix
        transition_matrix.sum_axis(Axis(1)).to_vec()
    }

    /// Get the accumulated child-parent cardinality ratio of each `OddBall` in the `Component`.
    pub fn accumulated_cp_car_ratios(&self) -> &[f32] {
        &self.accumulated_cp_car_ratios
    }
}

impl<U: Number> Index<usize> for Component<U> {
    type Output = (usize, usize, usize);

    fn index(&self, index: usize) -> &Self::Output {
        &self.clusters[index]
    }
}
