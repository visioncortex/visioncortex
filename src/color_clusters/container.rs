use crate::{Color, PointI32};
use super::Cluster;

#[derive(Copy, Clone, Default, Ord, PartialOrd, Eq, PartialEq)]
pub struct ClusterIndex(pub u32);

pub struct Clusters {
    pub width: u32,
    pub height: u32,
    pub(crate) pixels: Vec<u8>,
    pub(crate) clusters: Vec<Cluster>,
    pub(crate) cluster_indices: Vec<ClusterIndex>,
    pub(crate) clusters_output: Vec<ClusterIndex>, // valid outputs. Valid outputs are clusters with at least one pixel.
}

impl Clusters {
    pub fn view(&self) -> ClustersView {
        ClustersView {
            width: self.width,
            height: self.height,
            pixels: &self.pixels,
            clusters: &self.clusters,
            cluster_indices: &self.cluster_indices,
            clusters_output: &self.clusters_output,
        }
    }
    // todo: mutators
}

pub struct ClustersView<'a> {
    pub width: u32,
    pub height: u32,
    pub pixels: &'a [u8],
    pub clusters: &'a [Cluster],
    pub cluster_indices: &'a [ClusterIndex],
    pub clusters_output: &'a [ClusterIndex],
}

impl ClustersView<'_> {
    pub fn get_cluster(&self, index: ClusterIndex) -> &Cluster {
        &self.clusters[index.0 as usize]
    }

    pub fn get_cluster_at_point(&self, point: PointI32) -> ClusterIndex {
        let index = (point.y * self.width as i32 + point.x) as u32;
        self.get_cluster_at(index)
    }

    pub fn get_cluster_at(&self, index: u32) -> ClusterIndex {
        self.cluster_indices[index as usize]
    }

    pub fn get_pixel(&self, x: i32, y: i32) -> Option<Color> {
        if x < 0 || y < 0 {
            return None;
        }
        if x as u32 >= self.width {
            return None;
        }
        let index = (y * self.width as i32 + x) as u32;
        self.get_pixel_at_index(index)
    }

    pub fn get_pixel_at_index(&self, index: u32) -> Option<Color> {
        let index = index as usize * 4;
        if index >= self.pixels.len() {
            return None;
        }
        let r = self.pixels[index];
        let g = self.pixels[index + 1];
        let b = self.pixels[index + 2];
        let a = self.pixels[index + 3];

        Some(Color::new_rgba(r, g, b, a))
    }
}
