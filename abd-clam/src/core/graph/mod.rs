//! Provides the `Graph` and `Edge` structs and the `MetaML` criteria for
//! building the `Graph`s.
#![allow(dead_code)]

mod _graph;
mod criteria;

#[allow(unused_imports)]
pub use _graph::{Edge, Graph};
pub use criteria::MetaMLScorer;
