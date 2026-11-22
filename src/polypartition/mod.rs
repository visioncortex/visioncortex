//! Polygon triangulation (ear-clipping, monotone, optimal dynamic programming)
//! and hole removal.
//!
//! Rust port of PolyPartition by Ivan Fratric
//! (<https://github.com/ivanfratric/polypartition>), via
//! <https://github.com/visioncortex/polypartition>. Licensed MIT; see the
//! `polypartition` section of `Attributions.md` for the original copyright.
//!
//! Unlike upstream, which assumes general position, every algorithm here
//! handles the degenerate input that outlines traced from pixels consist of:
//! collinear runs, diagonals through reflex corners, holes touching the
//! outline or each other at a corner or mid-edge, zero-width spikes and
//! hole bridges. Predicates are exact for integer coordinates, the monotone
//! sweep resolves ties by symbolic perturbation, and output on input in
//! general position is unchanged. Self-crossing input remains unsupported.

mod hole;
mod enums;
mod polygon;
mod triangulation;
mod util;
mod vertex;

pub use hole::*;
pub use enums::*;
pub use polygon::*;
pub use triangulation::*;
pub use util::*;
pub use vertex::*;