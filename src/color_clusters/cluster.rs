use crate::{BinaryImage, BoundingRect, Color, ColorSum, PointI32, PathSimplifyMode, Shape};
use crate::clusters::Cluster as BinaryCluster;
use super::container::{ClusterIndex, ClustersView};

#[derive(Clone, Default)]
pub struct Cluster {
    pub indices: Vec<u32>,
    pub residue: Vec<u32>,
    pub holes: Vec<u32>,
    pub num_holes: u32,
    pub depth: u32,
    pub sum: ColorSum,
    pub residue_sum: ColorSum,
    pub rect: BoundingRect,
    pub merged_into: Option<ClusterIndex>,
}

impl Cluster {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, i: u32, color: &Color, x: i32, y: i32) {
        self.indices.push(i);
        self.sum.add(color);
        self.rect.add_x_y(x, y);
    }

    pub fn area(&self) -> usize {
        self.indices.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &u32> {
        self.indices.iter()
    }

    pub fn color(&self) -> Color {
        self.sum.average()
    }

    pub fn residue_color(&self) -> Color {
        self.residue_sum.average()
    }

    pub fn perimeter(&self, parent: &ClustersView) -> u32 {
        let mut perimeter = 0;
        let index = parent.get_cluster_at(self.indices[0]);
        for &i in self.residue.iter() {
            let x = i % parent.width;
            let y = i / parent.width;
            for k in 0..4 {
                let c = match k {
                    0 => {
                        if y > 0 {
                            parent.cluster_indices[(parent.width * (y - 1) + x) as usize]
                        } else {
                            ClusterIndex(0)
                        }
                    }
                    1 => {
                        if y < parent.height - 1 {
                            parent.cluster_indices[(parent.width * (y + 1) + x) as usize]
                        } else {
                            ClusterIndex(0)
                        }
                    }
                    2 => {
                        if x > 0 {
                            parent.cluster_indices[(parent.width * y + (x - 1)) as usize]
                        } else {
                            ClusterIndex(0)
                        }
                    }
                    3 => {
                        if x < parent.width - 1 {
                            parent.cluster_indices[(parent.width * y + (x + 1)) as usize]
                        } else {
                            ClusterIndex(0)
                        }
                    }
                    _ => unreachable!(),
                };
                if c.0 != 0 && c != index {
                    perimeter += 1; 
                }
            }
        }
        perimeter
    }

    pub fn to_image(&self, parent: &ClustersView) -> BinaryImage {
        self.to_image_with_hole(parent, true)
    }

    pub fn to_image_with_hole(&self, parent: &ClustersView, hole: bool) -> BinaryImage {
        let width = self.rect.width() as usize;
        let height = self.rect.height() as usize;
        let mut image = BinaryImage::new_w_h(width, height);

        for &i in self.iter() {
            let x = (i as i32 % parent.width as i32) - self.rect.left;
            let y = (i as i32 / parent.width as i32) - self.rect.top;
            image.set_pixel(x as usize, y as usize, true);
        }

        if hole {
            for &i in self.holes.iter() {
                let x = (i as i32 % parent.width as i32) - self.rect.left;
                let y = (i as i32 / parent.width as i32) - self.rect.top;
                image.set_pixel(x as usize, y as usize, false);
            }
        }

        image
    }

    pub fn to_shape(&self, parent: &ClustersView) -> Shape {
        self.to_image(parent).into()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn to_svg_path(&self,
        parent: &ClustersView,
        hole: bool,
        mode: PathSimplifyMode,
        corner_threshold: f64,
        length_threshold: f64,
        max_iterations: usize,
        splice_threshold: f64
    ) -> String {
        let origin = PointI32 {
            x: self.rect.left,
            y: self.rect.top,
        };
        let mut svg_paths = vec![];
        for cluster in self.to_image_with_hole(parent, hole).to_clusters(false).iter() {
            svg_paths.push(
                BinaryCluster::svg_path_static(&PointI32 {
                    x: origin.x + cluster.rect.left,
                    y: origin.y + cluster.rect.top,
				}, &cluster.to_binary_image(), mode,
				corner_threshold, length_threshold, max_iterations, splice_threshold)
            );
        }
        svg_paths.concat()
    }
}
