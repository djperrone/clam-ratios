//! Provides the `Graph` and `Edge` structs.

use core::hash::{Hash, Hasher};

use std::collections::{HashMap, HashSet};

use distances::Number;

use crate::Cluster;

/// A `HashSet` of `Cluster`s.
type ClusterSet<'a, T, U> = HashSet<&'a Cluster<T, U>>;
/// A `HashSet` of `Edge`s.
type EdgeSet<'a, T, U> = HashSet<&'a Edge<'a, T, U>>;
/// A `HashMap` from `Cluster`s to a `HashSet` of their neighbors.
type AdjacencyMap<'a, T, U> = HashMap<&'a Cluster<T, U>, ClusterSet<'a, T, U>>;
/// A `HashMap` from `Cluster`s to a `Vec` of their frontier size at each step
/// during a graph traversal.
type FrontierSizes<'a, T, U> = HashMap<&'a Cluster<T, U>, Vec<usize>>;

/// Two `Cluster`s have an `Edge` between them if they have overlapping volumes.
///
/// In CLAM, all `Edge`s are bi-directional.
#[derive(Debug, Clone)]
pub struct Edge<'a, T: Send + Sync + Copy, U: Number> {
    /// The `Cluster` at the `left` end of the `Edge`.
    pub left: &'a Cluster<T, U>,
    /// The `Cluster` at the `right` end of the `Edge`.
    pub right: &'a Cluster<T, U>,
    /// The distance between the two `Cluster`s connected by this `Edge`.
    pub distance: U,
}

impl<'a, T: Send + Sync + Copy, U: Number> PartialEq for Edge<'a, T, U> {
    fn eq(&self, other: &Self) -> bool {
        (self.left == other.left) && (self.right == other.right)
    }
}

/// Two `Edge`s are equal if they connect the same two `Cluster`s.
impl<'a, T: Send + Sync + Copy, U: Number> Eq for Edge<'a, T, U> {}

impl<'a, T: Send + Sync + Copy, U: Number> std::fmt::Display for Edge<'a, T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:} -- {:}", self.left, self.right)
    }
}

impl<'a, T: Send + Sync + Copy, U: Number> Hash for Edge<'a, T, U> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        format!("{self}").hash(state);
    }
}

impl<'a, T: Send + Sync + Copy, U: Number> Edge<'a, T, U> {
    /// Creates a new `Edge` from the given `Cluster`s and the distance between
    /// them.
    ///
    /// It is upon the user to verify that the two `Cluster`s are close enough
    /// to have an edge between them.
    pub fn new(left: &'a Cluster<T, U>, right: &'a Cluster<T, U>, distance: U) -> Self {
        if left < right {
            Self { left, right, distance }
        } else {
            Self {
                left: right,
                right: left,
                distance,
            }
        }
    }

    /// Whether this edge has the given `Cluster` at one of its ends.
    pub fn contains(&self, c: &Cluster<T, U>) -> bool {
        c == self.left || c == self.right
    }

    /// A 2-slice of the `Cluster`s in this `Edge`.
    pub const fn clusters(&self) -> [&Cluster<T, U>; 2] {
        [self.left, self.right]
    }

    /// Whether this is an edge from a `Cluster` to itself.
    pub fn is_circular(&self) -> bool {
        self.left == self.right
    }

    /// Returns the neighbor of the given `Cluster` in this `Edge`.
    ///
    /// Err:
    ///
    /// * If `c` is not one of the `Cluster`s connected by this `Edge`.
    pub fn neighbor(&self, c: &Cluster<T, U>) -> Result<&Cluster<T, U>, String> {
        if c == self.left {
            Ok(self.right)
        } else if c == self.right {
            Ok(self.left)
        } else {
            Err(format!("Cluster {c} is not in this edge {self}."))
        }
    }
}

/// A `Graph` represents a collection of `Cluster`s and `Edge`s, i.e.
/// connections between overlapping `Cluster`s.
///
/// TODO: Add more info on what graphs we useful for.
#[derive(Debug, Clone)]
pub struct Graph<'a, T: Send + Sync + Copy, U: Number> {
    /// The set of `Cluster`s in this `Graph`.
    pub clusters: ClusterSet<'a, T, U>,
    /// The set of `Edge`s in this `Graph`.
    pub edges: EdgeSet<'a, T, U>,
    /// A `HashMap` from `Cluster`s to a `HashSet` of their neighbors.
    pub adjacency_map: AdjacencyMap<'a, T, U>,
    /// The total number of points in all `Cluster`s in this `Graph`.
    pub population: usize,
    /// The minimum depth of any `Cluster` in this `Graph`.
    pub min_depth: usize,
    /// The maximum depth of any `Cluster` in this `Graph`.
    pub max_depth: usize,
    /// The `Cluster`s in this `Graph` in sorted order.
    pub ordered_clusters: Vec<&'a Cluster<T, U>>,
    /// The square distance matrix for this `Graph`.
    pub distance_matrix: Option<Vec<Vec<U>>>,
    /// The adjacency matrix for this `Graph`.
    pub adjacency_matrix: Option<Vec<Vec<bool>>>,
    /// The frontier sizes for each `Cluster` in this `Graph`.
    pub frontier_sizes: Option<FrontierSizes<'a, T, U>>, // TODO: Bench when replacing with DashMap
}

impl<'a, T: Send + Sync + Copy, U: Number> Graph<'a, T, U> {
    /// Create a new `Graph` from the given `clusters` and `edges`. The easiest
    /// and most efficient way to construct a graph is from methods in
    /// `Manifold`.
    ///
    /// # Arguments:
    ///
    /// * `clusters`: The set of `Cluster`s with which to build the `Graph`.
    /// * `edges`: The set of `Edge`s with which to build the `Graph`.
    pub fn new(clusters: ClusterSet<'a, T, U>, edges: EdgeSet<'a, T, U>) -> Self {
        assert!(!clusters.is_empty());

        let (population, min_depth, max_depth) =
            clusters
                .iter()
                .fold((0, usize::MAX, 0), |(population, min_depth, max_depth), &c| {
                    (
                        population + c.cardinality,
                        std::cmp::min(min_depth, c.depth()),
                        std::cmp::max(max_depth, c.depth()),
                    )
                });

        let adjacency_map = {
            let mut adjacency_map: AdjacencyMap<T, U> = clusters.iter().map(|&c| (c, HashSet::new())).collect();
            for &e in &edges {
                adjacency_map
                    .get_mut(e.left)
                    .unwrap_or_else(|| unreachable!("We added the Cluster ourselves"))
                    .insert(e.right);
                adjacency_map
                    .get_mut(e.right)
                    .unwrap_or_else(|| unreachable!("We added the Cluster ourselves"))
                    .insert(e.left);
            }
            adjacency_map
        };

        Self {
            ordered_clusters: clusters.iter().copied().collect(),
            clusters,
            edges,
            adjacency_map,
            population,
            min_depth,
            max_depth,
            distance_matrix: None,
            adjacency_matrix: None,
            frontier_sizes: None,
        }
    }

    /// Computes the distance matrix for the `Graph`.
    fn compute_distance_matrix(&self) -> Vec<Vec<U>> {
        let indices: HashMap<_, _> = self.ordered_clusters.iter().enumerate().map(|(i, &c)| (c, i)).collect();
        let mut matrix: Vec<Vec<U>> = vec![vec![U::zero(); self.vertex_cardinality()]; self.vertex_cardinality()];
        self.edges.iter().for_each(|&e| {
            let i = *indices
                .get(e.left)
                .unwrap_or_else(|| unreachable!("We added the Cluster ourselves"));
            let j = *indices
                .get(e.right)
                .unwrap_or_else(|| unreachable!("We added the Cluster ourselves"));
            matrix[i][j] = e.distance;
            matrix[j][i] = e.distance;
        });
        matrix
    }

    /// Computes the distance matrix for the `Graph` and stores it as an
    /// internal property.
    pub fn with_distance_matrix(mut self) -> Self {
        self.distance_matrix = Some(self.compute_distance_matrix());
        self
    }

    /// Computes the adjacency matrix for the `Graph` and stores it as an
    /// internal property.
    ///
    /// # Panics:
    ///
    /// * If called before calling `with_distance_matrix`.
    pub fn with_adjacency_matrix(mut self) -> Self {
        self.adjacency_matrix = Some(
            self.distance_matrix
                .as_ref()
                .unwrap_or_else(|| unreachable!("Please call `with_distance_matrix` before using this method."))
                .iter()
                .map(|row| row.iter().map(|&v| v != U::zero()).collect())
                .collect(),
        );
        self
    }

    /// Computes the eccentricity of each `Cluster` and stores it in the `Graph`.
    pub fn with_eccentricities(&'a self) -> Self {
        let frontier_sizes = Some(
            self.clusters
                .iter()
                .map(|&c| (c, self.unchecked_traverse(c).1))
                .collect(),
        );

        Self {
            clusters: self.clusters.clone(),
            edges: self.edges.clone(),
            adjacency_map: self.adjacency_map.clone(),
            population: self.population,
            min_depth: self.min_depth,
            max_depth: self.max_depth,
            ordered_clusters: self.ordered_clusters.clone(),
            distance_matrix: self.distance_matrix.clone(),
            adjacency_matrix: self.adjacency_matrix.clone(),
            frontier_sizes,
        }
    }

    /// Returns the `Cluster`s in each connected component of the `Graph`.
    #[allow(clippy::manual_retain)]
    pub fn find_component_clusters(&'a self) -> Vec<ClusterSet<'a, T, U>> {
        let mut components = Vec::new();

        let mut unvisited = self.clusters.clone();
        while !unvisited.is_empty() {
            let &start = unvisited
                .iter()
                .next()
                .unwrap_or_else(|| unreachable!("We know there is at least one unvisited Cluster"));
            let (visited, _) = self.unchecked_traverse(start);

            // TODO: bench this using `unvisited.retain(|c| !visited.contains(c))`
            unvisited = unvisited.into_iter().filter(|&c| !visited.contains(c)).collect();

            // TODO: Also grab adjacency map, distance matrix, and adjacency matrix
            components.push(visited);
        }

        components
    }

    /// Returns the number of `Cluster`s in the `Graph`.
    pub fn vertex_cardinality(&self) -> usize {
        self.clusters.len()
    }

    /// Returns the number of `Edge`s in the `Graph`.
    pub fn edge_cardinality(&self) -> usize {
        self.edges.len()
    }

    /// Returns the minimum and maximum depth of any `Cluster` as a 2-tuple.
    pub const fn depth_range(&self) -> (usize, usize) {
        (self.min_depth, self.max_depth)
    }

    /// Returns the `Graph` diameter, i.e. the maximum eccentricity of any
    /// `Cluster`.
    pub fn diameter(&'a self) -> usize {
        self.clusters
            .iter()
            .map(|&c| self.unchecked_eccentricity(c))
            .max()
            .unwrap_or_else(|| unreachable!("We know there is at least one Cluster"))
    }

    /// Checks whether the given `Cluster` is in this `Graph`.
    fn assert_contains(&self, c: &Cluster<T, U>) -> Result<(), String> {
        if self.clusters.contains(&c) {
            Ok(())
        } else {
            Err(format!("Cluster {c} is not in this graph."))
        }
    }

    /// Returns the degree of the `Cluster`.
    pub fn unchecked_vertex_degree(&'a self, c: &Cluster<T, U>) -> usize {
        self.unchecked_neighbors_of(c).len()
    }

    /// Returns the degree of the `Cluster`.
    pub fn vertex_degree(&'a self, c: &Cluster<T, U>) -> Result<usize, String> {
        self.assert_contains(c)?;
        Ok(self.unchecked_vertex_degree(c))
    }

    /// Returns the `Cluster`s adjacent to the given `Cluster`.
    pub fn unchecked_neighbors_of(&'a self, c: &Cluster<T, U>) -> &ClusterSet<T, U> {
        self.adjacency_map
            .get(c)
            .unwrap_or_else(|| unreachable!("Please call this with a Cluster that is in the Graph."))
    }

    /// Returns the `Cluster`s adjacent to the given `Cluster`.
    pub fn neighbors_of(&'a self, c: &Cluster<T, U>) -> Result<&ClusterSet<T, U>, String> {
        self.assert_contains(c)?;
        Ok(self.unchecked_neighbors_of(c))
    }

    /// Preforms a `Graph` traversal starting at the given `Cluster` and returns
    /// the `Cluster`s visited and the frontier sizes at each step.
    pub fn unchecked_traverse(&'a self, start: &'a Cluster<T, U>) -> (ClusterSet<T, U>, Vec<usize>) {
        let mut visited: HashSet<&Cluster<T, U>> = HashSet::new();
        let mut frontier: HashSet<&Cluster<T, U>> = HashSet::new();
        frontier.insert(start);
        let mut frontier_sizes: Vec<usize> = Vec::new();

        while !frontier.is_empty() {
            visited.extend(frontier.iter().copied());
            frontier = frontier
                .iter()
                .flat_map(|&c| self.unchecked_neighbors_of(c))
                .filter(|&n| !((visited.contains(n)) || (frontier.contains(n))))
                .copied()
                .collect();
            frontier_sizes.push(frontier.len());
        }

        (visited, frontier_sizes)
    }

    /// Preforms a `Graph` traversal starting at the given `Cluster` and returns
    /// the `Cluster`s visited and the frontier sizes at each step.
    #[allow(clippy::type_complexity)]
    pub fn traverse(&'a self, start: &'a Cluster<T, U>) -> Result<(ClusterSet<T, U>, Vec<usize>), String> {
        self.assert_contains(start)?;
        Ok(self.unchecked_traverse(start))
    }

    /// Returns the frontier sizes for the given `Cluster`.
    pub fn unchecked_frontier_sizes(&'a self, c: &'a Cluster<T, U>) -> &[usize] {
        self.frontier_sizes
            .as_ref()
            .unwrap_or_else(|| unreachable!("Please call `with_eccentricities` before using this method."))
            .get(c)
            .unwrap_or_else(|| unreachable!("Please call this with a Cluster that is in the Graph."))
    }

    /// Returns the frontier sizes for the given `Cluster`.
    pub fn frontier_sizes(&'a self, c: &'a Cluster<T, U>) -> Result<&[usize], String> {
        self.assert_contains(c)?;
        Ok(self.unchecked_frontier_sizes(c))
    }

    /// Returns the eccentricity of the given `Cluster`.
    pub fn unchecked_eccentricity(&'a self, c: &'a Cluster<T, U>) -> usize {
        self.unchecked_frontier_sizes(c).len()
    }

    /// Returns the eccentricity of the given `Cluster`.
    pub fn eccentricity(&'a self, c: &'a Cluster<T, U>) -> Result<usize, String> {
        self.assert_contains(c)?;
        Ok(self.unchecked_eccentricity(c))
    }
}
