//! Algorithm to build an image binary tree
//! 
//! The hierarchical structure resembles the human visual cortex.
//! 
//! To support interactivity, components follow a state-machine model:
//! 
//! + new(): creation of placeholder object
//! + init(): resource allocation
//! + tick() -> bool: computation. returning false to continue, returning true when finish
//! + result() -> T: cleanup & collect results

mod builder;
mod cluster;
mod container;
mod runner;

pub use builder::*;
pub use cluster::*;
pub use container::*;
pub use runner::*;