#![allow(dead_code)]

//! Utility functions for tests.

use core::cmp::Ordering;

use crate::VecDataset;
use distances::{
    number::{Float, UInt},
    Number,
};

/// Euclidean distance between two vectors.
pub fn euclidean<T: Number, F: Float>(x: &Vec<T>, y: &Vec<T>) -> F {
    distances::vectors::euclidean(x, y)
}

/// Euclidean distance between two vectors.
pub fn euclidean_sq<T: Number>(x: &Vec<T>, y: &Vec<T>) -> T {
    distances::vectors::euclidean_sq(x, y)
}

/// Hamming distance between two Strings.
pub fn hamming<T: UInt>(x: &String, y: &String) -> T {
    distances::strings::hamming(x, y)
}

/// Levenshtein distance between two Strings.
pub fn levenshtein<T: UInt>(x: &String, y: &String) -> T {
    distances::strings::levenshtein(x, y)
}

/// Needleman-Wunsch distance between two Strings.
pub fn needleman_wunsch<T: UInt>(x: &String, y: &String) -> T {
    distances::strings::needleman_wunsch::nw_distance(x, y)
}
