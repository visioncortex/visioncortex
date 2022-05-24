use crate::{Color, ColorImage, PointI32};
use super::Cluster;

pub struct Clusters {
    pub width: u32,
    pub height: u32,
    pub(crate) pixels: Vec<u8>,
    pub(crate) clusters: Vec<Cluster>,
    pub(crate) cluster_indices: Vec<ClusterIndex>,
    pub(crate) clusters_output: Vec<ClusterIndex>, // valid outputs. Valid outputs are clusters with at least one pixel.
}

#[derive(Copy, Clone, Default, Eq, Ord, Hash, PartialEq, PartialOrd)]
pub struct ClusterIndex(pub ClusterIndexElem);

pub type ClusterIndexElem = u32;

impl Clusters {
    pub fn output_len(&self) -> usize {
        self.clusters_output.len()
    }

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

    pub fn take_image(self) -> ColorImage {
        ColorImage {
            pixels: self.pixels,
            width: self.width as usize,
            height: self.height as usize,
        }
    }
}
#[derive(Copy, Clone)]
pub struct ClustersView<'a> {
    pub width: u32,
    pub height: u32,
    pub pixels: &'a [u8],
    pub clusters: &'a [Cluster],
    pub cluster_indices: &'a [ClusterIndex],
    pub clusters_output: &'a [ClusterIndex],
}

pub struct ClustersOutputIterator<'a> {
    counter: usize,
    total: usize,
    parent: &'a ClustersView<'a>,
}

impl ClustersView<'_> {
    pub fn iter(&self) -> impl Iterator<Item = &Cluster> {
        ClustersOutputIterator {
            counter: 0,
            total: self.clusters_output.len(),
            parent: self,
        }
    }

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

    pub fn to_color_image(&self) -> ColorImage {
        let mut image = ColorImage::new_w_h(self.width as usize, self.height as usize);

        self.clusters_output
            .iter()
            .rev()
            .for_each(|&u| {
                let cluster = self.get_cluster(u);
                cluster.render_to_color_image(self, &mut image);
            });

        image
    }
}

impl<'a> Iterator for ClustersOutputIterator<'a> {
    type Item = &'a Cluster;

    fn next(&mut self) -> Option<Self::Item> {
        if self.counter < self.total {
            let index = self.parent.clusters_output[self.counter];
            let cluster = self.parent.get_cluster(index);
            self.counter += 1;
            Some(cluster)
        } else {
            None
        }
    }
}