use std::collections::HashSet;
use crate::{BinaryImage, BoundingRect, Color, ColorImage, ColorSum, CompoundPath, PointI32, PathSimplifyMode, Shape};
use crate::clusters::Cluster as BinaryCluster;
use super::container::{ClusterIndex, ClustersView};
use super::builder::{BuilderImpl, ZERO};

#[derive(Clone, Default)]
pub struct Cluster {
    pub indices: Vec<u32>,
    pub holes: Vec<u32>,
    pub num_holes: u32,
    pub depth: u32,
    pub sum: ColorSum,
    pub residue_sum: ColorSum,
    pub rect: BoundingRect,
    pub merged_into: ClusterIndex,
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
        Shape::image_boundary_list(&self.to_image(parent)).len() as u32
    }
    pub(crate) fn perimeter_internal(&self, internal: &BuilderImpl) -> u32 {
        Shape::image_boundary_list(&self.to_image_internal(internal)).len() as u32
    }

    pub fn to_image(&self, parent: &ClustersView) -> BinaryImage {
        self.to_image_with_hole(parent.width, true)
    }
    fn to_image_internal(&self, internal: &BuilderImpl) -> BinaryImage {
        self.to_image_with_hole(internal.width, true)
    }

    pub fn to_image_with_hole(&self, width: u32, hole: bool) -> BinaryImage {
        let width = self.rect.width() as usize;
        let height = self.rect.height() as usize;
        let mut image = BinaryImage::new_w_h(width, height);

        for &i in self.iter() {
            let x = (i as i32 % width as i32) - self.rect.left;
            let y = (i as i32 / width as i32) - self.rect.top;
            image.set_pixel(x as usize, y as usize, true);
        }

        if hole {
            for &i in self.holes.iter() {
                let x = (i as i32 % width as i32) - self.rect.left;
                let y = (i as i32 / width as i32) - self.rect.top;
                image.set_pixel(x as usize, y as usize, false);
            }
        }

        image
    }

    pub fn render_to_binary_image(&self, parent: &ClustersView, image: &mut BinaryImage) {
        for &i in self.iter() {
            let x = i % parent.width;
            let y = i / parent.width;
            image.set_pixel(x as usize, y as usize, true);
        }
    }

    pub fn render_to_color_image(&self, parent: &ClustersView, image: &mut ColorImage) {
        let color = self.residue_color();
        self.render_to_color_image_with_color(parent, image, &color);
    }

    pub fn render_to_color_image_with_color(&self, parent: &ClustersView, image: &mut ColorImage, color: &Color) {
        for &i in self.iter() {
            let x = i % parent.width;
            let y = i / parent.width;
            image.set_pixel(x as usize, y as usize, &color);
        }
    }

    pub fn to_shape(&self, parent: &ClustersView) -> Shape {
        self.to_image(parent).into()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn to_compound_path(&self,
        parent: &ClustersView,
        hole: bool,
        mode: PathSimplifyMode,
        corner_threshold: f64,
        length_threshold: f64,
        max_iterations: usize,
        splice_threshold: f64
    ) -> CompoundPath {
        let mut paths = CompoundPath::new();
        for cluster in self.to_image_with_hole(parent.width, hole).to_clusters(false).iter() {
            paths.append(
                BinaryCluster::image_to_compound_path(&PointI32 {
                    x: self.rect.left + cluster.rect.left,
                    y: self.rect.top + cluster.rect.top,
                }, &cluster.to_binary_image(), mode,
                corner_threshold, length_threshold, max_iterations, splice_threshold)
            );
        }
        paths
    }

    pub fn neighbours(&self, parent: &ClustersView) -> Vec<ClusterIndex> {
        let myself = parent.get_cluster_at(*self.indices.first().unwrap());
        let mut neighbours = HashSet::new();

        for &i in self.iter() {
            let x = i % parent.width;
            let y = i / parent.width;

            for k in 0..4 {
                let index = match k {
                    0 => if y > 0 { parent.cluster_indices[(parent.width * (y - 1) + x) as usize] } else { ZERO },
                    1 => if y < parent.height - 1 { parent.cluster_indices[(parent.width * (y + 1) + x) as usize] } else { ZERO },
                    2 => if x > 0 { parent.cluster_indices[(parent.width * y + (x - 1)) as usize] } else { ZERO },
                    3 => if x < parent.width - 1 { parent.cluster_indices[(parent.width * y + (x + 1)) as usize] } else { ZERO },
                    _ => unreachable!(),
                };
                if index != ZERO && index != myself {
                    neighbours.insert(index);
                }
            }
        }

        let mut list: Vec<ClusterIndex> = neighbours.into_iter().collect();
        list.sort();
        list
    }

    /// Equivalent to [`neighbours()`] but operates on `BuilderImpl` directly, 
    /// removing the overhead of constructing a `ClustersView`
    pub(crate) fn neighbours_internal(&self, internal: &BuilderImpl) -> Vec<ClusterIndex> {
        let myself = internal.cluster_indices[*self.indices.first().unwrap() as usize];
        let mut neighbours = HashSet::new();

        for &i in self.iter() {
            let x = i % internal.width;
            let y = i / internal.width;

            for k in 0..4 {
                let index = match k {
                    0 => if y > 0 { internal.cluster_indices[(internal.width * (y - 1) + x) as usize] } else { ZERO },
                    1 => if y < internal.height - 1 { internal.cluster_indices[(internal.width * (y + 1) + x) as usize] } else { ZERO },
                    2 => if x > 0 { internal.cluster_indices[(internal.width * y + (x - 1)) as usize] } else { ZERO },
                    3 => if x < internal.width - 1 { internal.cluster_indices[(internal.width * y + (x + 1)) as usize] } else { ZERO },
                    _ => unreachable!(),
                };
                if index != ZERO && index != myself {
                    neighbours.insert(index);
                }
            }
        }

        let mut list: Vec<ClusterIndex> = neighbours.into_iter().collect();
        list.sort();
        list
    }
}
