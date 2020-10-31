use crate::{BinaryImage, Bound, BoundingRect};

/// An artificial object that have a shape and a position in space
#[derive(Debug, Default)]
pub struct Artifact {
    /// shape image
    pub image: BinaryImage,
    /// position in space
    pub bound: BoundingRect,
}

impl Artifact {
    pub fn new(rect: BoundingRect, image: BinaryImage) -> Self {
        Self {
            bound: rect,
            image,
        }
    }
}

impl Bound for Artifact {
    fn bound(&self) -> BoundingRect {
        self.bound
    }
}
