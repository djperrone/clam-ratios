//! Criteria used for selecting `Cluster`s for `Graph`s.

/// A function that assigns a score for a given `Cluster` using the `Ratios` of
/// the `Cluster`.
pub type MetaMLScorer = fn(crate::cluster::Ratios) -> f64;
