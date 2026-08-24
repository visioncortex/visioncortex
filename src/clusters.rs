//! Algorithm to cluster a binary image

use crate::{BinaryImage, BoundingRect, CompoundPath, PathI32, PathSimplifyMode, PointI32, ScalarField, Shape, Spline};

/// Provisional label handed out by the scanline pass of [`BinaryImage::to_clusters`].
///
/// A label is taken every time a pixel starts a run with no labelled neighbour, and
/// only a merge into the most recently issued label ever gives one back, so the peak
/// tracks the number of runs rather than the number of connected components. A finely
/// dithered mask can need one per set pixel, which is why this is not `MonoImageItem`.
type ClusterLabel = u32;
type ClusterLabelMap = ScalarField<ClusterLabel>;

/// A cluster of binary image pixels
#[derive(Default)]
pub struct Cluster {
    /// Points are in absolute coordinate, i.e. (0, 0) is the coordinate of the left-top corner of the raw frame.
    pub points: Vec<PointI32>,
    pub rect: BoundingRect,
}

/// A collection of clusters
#[derive(Default)]
pub struct Clusters {
    pub clusters: Vec<Cluster>,
    pub rect: BoundingRect,
}

impl Cluster {
    pub fn iter(&self) -> std::slice::Iter<'_, PointI32> {
        self.points.iter()
    }

    pub fn add(&mut self, pos: PointI32) {
        self.points.push(pos);
        self.rect.add_x_y(pos.x as i32, pos.y as i32);
    }

    pub fn size(&self) -> usize {
        self.points.len()
    }

    pub fn to_binary_image(&self) -> BinaryImage {
        let mut image =
            BinaryImage::new_w_h(self.rect.width() as usize, self.rect.height() as usize);
        for p in self.points.iter() {
            image.set_pixel(
                p.x as usize - self.rect.left as usize,
                p.y as usize - self.rect.top as usize,
                true,
            );
        }
        image
    }

    pub fn boundary(&self) -> Vec<PointI32> {
        Shape::image_boundary_list(&self.to_binary_image())
    }

    pub fn offset(&mut self, o: PointI32) {
        for p in self.points.iter_mut() {
            *p += o;
        }
        self.rect.translate(o);
    }

    pub fn to_compound_path(
        &self,
        mode: PathSimplifyMode,
        corner_threshold: f64,
        segment_length: f64,
        max_iterations: usize,
        splice_threshold: f64
    ) -> CompoundPath {
        let origin = PointI32 {
            x: self.rect.left,
            y: self.rect.top,
        };
        Self::image_to_compound_path(
            &origin,
            &self.to_binary_image(),
            mode,
            corner_threshold,
            segment_length,
            max_iterations,
            splice_threshold
        )
    }

    pub fn image_to_compound_path(
        offset: &PointI32,
        image: &BinaryImage,
        mode: PathSimplifyMode,
        corner_threshold: f64,
        segment_length: f64,
        max_iterations: usize,
        splice_threshold: f64
    ) -> CompoundPath {
        match mode {
            PathSimplifyMode::None | PathSimplifyMode::Polygon => {
                let paths = Self::image_to_paths(image, mode);
                let mut group = CompoundPath::new();
                for mut path in paths.into_iter() {
                    path.offset(&offset);
                    group.add_path_i32(path);
                }
                group
            },
            PathSimplifyMode::Spline => {
                let splines = Self::image_to_splines(image, corner_threshold, segment_length, max_iterations, splice_threshold);
                let mut group = CompoundPath::new();
                for mut spline in splines.into_iter() {
                    spline.offset(&offset.to_point_f64());
                    group.add_spline(spline);
                }
                group
            },
        }
    }

    pub fn image_to_paths(image: &BinaryImage, mode: PathSimplifyMode) -> Vec<PathI32> {
        let mut boundaries = image_boundaries(image);
        let mut paths = vec![];
        for (i, (image, offset)) in boundaries.iter_mut().enumerate() {
            let mut path = PathI32::image_to_path(image, i == 0, mode);
            path.offset(offset);
            if !path.is_empty() {
                paths.push(path);
            }
        }
        paths
    }

    const OUTSET_RATIO: f64 = 8.0;

    pub fn image_to_splines(image: &BinaryImage, corner_threshold: f64, segment_length: f64, max_iterations:usize, splice_threshold: f64) -> Vec<Spline> {
        let mut boundaries = image_boundaries(image);
        let mut splines = vec![];
        for (i, (image, offset)) in boundaries.iter_mut().enumerate() {
            let mut spline = Spline::from_image(
                image, i == 0, corner_threshold, Self::OUTSET_RATIO, segment_length, max_iterations, splice_threshold
            );
            spline.offset(&offset.to_point_f64());
            if !spline.is_empty() {
                splines.push(spline);
            }
        }
        splines
    }

    pub fn break_cluster(cluster: Cluster) -> Clusters {
        let mut clusters = Clusters::default();
        Self::break_cluster_recursive(cluster, &mut clusters);
        clusters
    }

    const BREAK_AT_LEAST: usize = 5;

    pub fn break_cluster_recursive(cluster: Cluster, output: &mut Clusters) {
        let mut image = cluster.to_binary_image();
        let (w, h) = (2, 3);
        let mut broke = false;
        if image.width >= w && image.height >= h {
            'outer: for y in 0..(image.height-h+1) {
                for x in 0..(image.width-w+1) {
                    if  image.get_pixel(x, y)   != image.get_pixel(x+1, y) &&
                        image.get_pixel(x, y+1) && image.get_pixel(x+1, y+1) &&
                        image.get_pixel(x, y+2) != image.get_pixel(x+1, y+2) &&
                        image.get_pixel(x, y)   == image.get_pixel(x+1, y+2) {
                        /*  either      or
                            *-          -*
                            **          **
                            -*          *-
                        */
                        if x < image.width - 2 && image.get_pixel(x + 2, y + 1) {
                            image.set_pixel(x + 1, y + 1, false);
                            broke = true;
                            break 'outer;
                        } else if x > 0 && image.get_pixel(x - 1, y + 1) {
                            image.set_pixel(x, y + 1, false);
                            broke = true;
                            break 'outer;
                        }
                    }
                }
            }
        }
        let clusters = image.to_clusters(false);
        if broke {
            let min = clusters.iter().map(|cc| cc.size()).min().unwrap();
            if min < Self::BREAK_AT_LEAST {
                broke = false;
            }
        }
        if broke {
            for mut cc in clusters.clusters.into_iter() {
                cc.offset(PointI32 {
                    x: cluster.rect.left,
                    y: cluster.rect.top,
                });
                Self::break_cluster_recursive(cc, output);
            }
        } else {
            output.add_cluster(cluster);
        }
    }
}

impl Clusters {
    pub fn iter(&self) -> std::slice::Iter<'_, Cluster> {
        self.clusters.iter()
    }

    pub fn len(&self) -> usize {
        self.clusters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.clusters.is_empty()
    }

    pub fn get_cluster(&self, index: usize) -> &Cluster {
        &self.clusters[index]
    }

    pub fn add_cluster(&mut self, cluster: Cluster) {
        self.rect.merge(cluster.rect);
        self.clusters.push(cluster);
    }
}

impl IntoIterator for Clusters {
    type IntoIter = std::vec::IntoIter<Cluster>;
    type Item = Cluster;

    fn into_iter(self) -> Self::IntoIter {
        self.clusters.into_iter()
    }
}

impl BinaryImage {
    /// Group the set pixels into connected components.
    ///
    /// Panics if the image has more pixels than [`ClusterLabel`] can index; every
    /// label issued is backed by a distinct pixel, so that bound is what keeps the
    /// scanline pass below from running out of labels.
    pub fn to_clusters(&self, diagonal: bool) -> Clusters {
        assert!(
            self.width * self.height <= ClusterLabel::MAX as usize,
            "image of {}x{} is too large to cluster",
            self.width, self.height
        );
        let mut clusters = Vec::<Cluster>::new();
        let mut rect = BoundingRect::default();
        let mut clustermap = ClusterLabelMap::new_w_h(self.width, self.height);
        let mut clusterindex: ClusterLabel = 0;
        for y in 0..self.height {
            for x in 0..self.width {
                let pos = PointI32 { x: x as i32, y: y as i32 };
                let v = self.get_pixel_safe(x as i32, y as i32);
                let v_up = self.get_pixel_safe(x as i32, y as i32-1);
                let v_left = self.get_pixel_safe(x as i32-1, y as i32);
                let v_up_left = self.get_pixel_safe(x as i32-1, y as i32-1);
                let mut cluster_up = if y > 0 { clustermap.get_pixel(x as usize, y as usize-1) } else { 0 };
                let mut cluster_left = if x > 0 { clustermap.get_pixel(x as usize-1, y as usize) } else { 0 };
                let cluster_up_left = if x > 0 && y > 0 { clustermap.get_pixel(x as usize-1, y as usize-1) } else { 0 };
                if (v || diagonal) && v_up && v_left && cluster_left != cluster_up {
                    if clusters[cluster_left as usize].size() <= clusters[cluster_up as usize].size() {
                        combine_cluster(&mut clusters, &mut clustermap, cluster_left, cluster_up);
                        if clusterindex > 0 &&
                            cluster_left == clusterindex - 1 &&
                            clusterindex as usize == clusters.len() {
                            // reduce cluster counts
                            clusterindex -= 1;
                        }
                        cluster_left = cluster_up;
                    } else {
                        combine_cluster(&mut clusters, &mut clustermap, cluster_up, cluster_left);
                        cluster_up = cluster_left;
                    }
                }
                if v {
                    rect.add_x_y(x as i32, y as i32);
                    if v_up {
                        clustermap.set_pixel(x as usize, y as usize, cluster_up);
                        clusters[cluster_up as usize].add(pos);
                    } else if v_left {
                        clustermap.set_pixel(x as usize, y as usize, cluster_left);
                        clusters[cluster_left as usize].add(pos);
                    } else if v_up_left && diagonal {
                        clustermap.set_pixel(x as usize, y as usize, cluster_up_left);
                        clusters[cluster_up_left as usize].add(pos);
                    } else {
                        let mut newcluster = Cluster::default();
                        newcluster.add(pos);
                        if (clusterindex as usize) < clusters.len() {
                            clusters[clusterindex as usize] = newcluster;
                        } else {
                            clusters.push(newcluster);
                        }
                        clustermap.set_pixel(x as usize, y as usize, clusterindex);
                        clusterindex += 1;
                    }
                }
            }
        }

        pub fn combine_cluster(
            clusters: &mut Vec<Cluster>,
            clustermap: &mut ClusterLabelMap,
            from: ClusterLabel,
            to: ClusterLabel,
        ) {
            for o in clusters[from as usize].points.iter() {
                clustermap.set_pixel(o.x as usize, o.y as usize, to);
            }
            let mut drain = std::mem::replace(&mut clusters[from as usize].points, Vec::new());
            clusters[to as usize].points.append(&mut drain); // drain is now empty
            let rect = clusters[from as usize].rect;
            clusters[to as usize].rect.merge(rect);
        }

        let clusters = clusters.into_iter().filter(|c| c.size() != 0).collect();

        Clusters { clusters, rect }
    }
}

/// Label of a background cluster. Stored in the label map as `label + 1`, so that
/// `0` marks an ink pixel.
type Label = u32;

/// The background clusters found so far by [`cluster_background`].
///
/// A cluster is a pixel count and a bounding rect, named by a [`Label`]. When the scan
/// discovers that two labels name the same region it merges one into the other, and the
/// merges form a union-find forest that [`Scan::find`] walks to reach the cluster a label
/// now belongs to. Labels are never reused, so a label read from the label map stays valid
/// however many merges have happened since it was written there.
///
/// Clusters are returned by [`Scan::clusters`] in the order they were created, which is the
/// order of the top left pixel of each.
#[derive(Default)]
struct Scan {
    /// The label each label was merged into, or the label itself if it is a root.
    parent: Vec<Label>,
    /// Pixels in each cluster; `0` once it has been merged into another.
    size: Vec<usize>,
    /// Bounding rect of each cluster.
    rect: Vec<BoundingRect>,
}

impl Scan {
    fn find(&mut self, mut label: Label) -> Label {
        while self.parent[label as usize] != label {
            let grandparent = self.parent[self.parent[label as usize] as usize];
            self.parent[label as usize] = grandparent;
            label = grandparent;
        }
        label
    }

    fn combine(&mut self, from: Label, to: Label) {
        self.parent[from as usize] = to;
        self.size[to as usize] += self.size[from as usize];
        self.size[from as usize] = 0;
        let from_rect = self.rect[from as usize];
        self.rect[to as usize].merge(from_rect);
    }

    fn new_cluster(&mut self) -> Label {
        let label = self.parent.len() as Label;
        if label == Label::max_value() {
            panic!("overflow");
        }
        self.parent.push(label);
        self.size.push(0);
        self.rect.push(BoundingRect::default());
        label
    }

    fn add(&mut self, label: Label, x: usize, y: usize) {
        self.size[label as usize] += 1;
        self.rect[label as usize].add_x_y(x as i32, y as i32);
    }

    /// Label and bounding rect of every cluster that survived the scan, in the order they
    /// were created.
    fn clusters(&self) -> impl Iterator<Item = (Label, BoundingRect)> + use<'_> {
        (0..self.parent.len() as Label)
            .filter(|&label| self.parent[label as usize] == label)
            .map(|label| (label, self.rect[label as usize]))
    }
}

/// Clusters the background of `image` — the pixels that are not ink — 4-connected. Returns
/// the clusters and a map holding one label per pixel, stored as `label + 1` so that `0` can
/// stand for an ink pixel.
///
/// The result of this is identical to calling `image.negative().to_clusters(false)`. However,
/// this differs in that it does not materialize the points of each cluster, which is wasted
/// effort.
fn cluster_background(image: &BinaryImage) -> (Scan, Vec<Label>) {
    let (width, height) = (image.width, image.height);
    let mut map = vec![0 as Label; width * height];
    let mut scan = Scan::default();

    for y in 0..height {
        for x in 0..width {
            if image.get_pixel(x, y) {
                continue;
            }

            let up = if y > 0 && !image.get_pixel(x, y - 1) {
                Some(scan.find(map[(y - 1) * width + x] - 1))
            } else {
                None
            };

            let left = if x > 0 && !image.get_pixel(x - 1, y) {
                Some(scan.find(map[y * width + x - 1] - 1))
            } else {
                None
            };

            let cluster = match (up, left) {
                (Some(up), Some(left)) if up != left => {
                    // Fold the smaller cluster into the larger, so that `find` stays shallow.
                    if scan.size[left as usize] <= scan.size[up as usize] {
                        scan.combine(left, up);
                        up
                    } else {
                        scan.combine(up, left);
                        left
                    }
                }
                (Some(up), _) => up,
                (None, Some(left)) => left,
                (None, None) => scan.new_cluster(),
            };

            map[y * width + x] = cluster + 1;
            scan.add(cluster, x, y);
        }
    }

    (scan, map)
}

/// The images whose boundaries make up the paths of `image`: the image itself with
/// its holes filled in, followed by each hole, cropped to its bounding rect.
///
/// A hole is a background cluster that does not touch the border of the image.
fn image_boundaries(image: &BinaryImage) -> Vec<(BinaryImage, PointI32)> {
    let (mut scan, map) = cluster_background(image);

    let mut boundaries = vec![(image.clone(), PointI32 { x: 0, y: 0 })];

    // Where a hole lands in `boundaries`, by label; 0 for a cluster that is not a hole.
    // The holes are drawn in one pass over the label map below, rather than a pass over
    // each hole's rect: holes nest, so an outer hole's rect can span most of the image and
    // walking rects would cover the same pixels again and again.
    let mut boundary_of = vec![0 as Label; scan.parent.len()];

    for (hole, rect) in scan.clusters() {
        if  rect.left as usize == 0 ||
            rect.top as usize == 0 ||
            rect.right as usize == image.width ||
            rect.bottom as usize == image.height {
            continue;
        }

        boundary_of[hole as usize] = boundaries.len() as Label;

        boundaries.push((
            BinaryImage::new_w_h(rect.width() as usize, rect.height() as usize),
            PointI32 {
                x: rect.left,
                y: rect.top,
            },
        ));
    }

    if boundaries.len() > 1 {
        let (filled, holes) = boundaries.split_at_mut(1);

        for y in 0..image.height {
            for x in 0..image.width {
                let label = map[y * image.width + x];
                if label == 0 {
                    continue;
                }

                let boundary = boundary_of[scan.find(label - 1) as usize];
                if boundary == 0 {
                    continue;
                }

                filled[0].0.set_pixel(x, y, true);

                let (hole_image, offset) = &mut holes[boundary as usize - 1];
                hole_image.set_pixel(
                    x - offset.x as usize,
                    y - offset.y as usize,
                    true,
                );
            }
        }
    }

    boundaries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clusters_3x3() {
        let size = 3;
        let mut image = BinaryImage::new_w_h(size, size);
        image.set_pixel(0, 0, true);
        image.set_pixel(1, 1, true);
        image.set_pixel(2, 2, true);
        let clusters = image.to_clusters(false);
        assert_eq!(clusters.len(), 3);
        assert_eq!(clusters.clusters[0].size(), 1);
        assert_eq!(clusters.clusters[0].points[0], PointI32 { x: 0, y: 0 });
        assert_eq!(clusters.clusters[1].size(), 1);
        assert_eq!(clusters.clusters[1].points[0], PointI32 { x: 1, y: 1 });
        assert_eq!(clusters.clusters[2].size(), 1);
        assert_eq!(clusters.clusters[2].points[0], PointI32 { x: 2, y: 2 });
        let mut rect = BoundingRect::default();
        rect.add_x_y(2, 2);
        assert_eq!(clusters.clusters[2].rect, rect);
        let bin = clusters.clusters[0].to_binary_image();
        assert_eq!(bin.width, 1);
        assert_eq!(bin.height, 1);
        assert_eq!(bin.get_pixel(0, 0), true);
    }

    #[test]
    fn clusters_3x3_diagonal() {
        let size = 3;
        let mut image = BinaryImage::new_w_h(size, size);
        image.set_pixel(0, 0, true);
        image.set_pixel(1, 1, true);
        image.set_pixel(2, 2, true);
        let clusters = image.to_clusters(true);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters.clusters[0].size(), 3);
        assert_eq!(clusters.clusters[0].points[0], PointI32 { x: 0, y: 0 });
        assert_eq!(clusters.clusters[0].points[1], PointI32 { x: 1, y: 1 });
        assert_eq!(clusters.clusters[0].points[2], PointI32 { x: 2, y: 2 });
    }

    #[test]
    fn clusters_4x4() {
        let size = 4;
        let mut image = BinaryImage::new_w_h(size, size);
        image.set_pixel(1, 1, true);
        image.set_pixel(1, 2, true);
        image.set_pixel(2, 1, true);
        image.set_pixel(2, 2, true);
        let clusters = image.to_clusters(false);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters.clusters[0].size(), 4);
        assert_eq!(clusters.clusters[0].rect.left, 1);
        assert_eq!(clusters.clusters[0].rect.top, 1);
        assert_eq!(clusters.clusters[0].rect.right, 3);
        assert_eq!(clusters.clusters[0].rect.bottom, 3);
        assert_eq!(clusters.clusters[0].rect.width(), 2);
        assert_eq!(clusters.clusters[0].rect.height(), 2);
        let bin = clusters.clusters[0].to_binary_image();
        assert_eq!(bin.width, 2);
        assert_eq!(bin.height, 2);
        assert_eq!(bin.get_pixel(0, 0), true);
        assert_eq!(bin.get_pixel(0, 1), true);
        assert_eq!(bin.get_pixel(1, 0), true);
        assert_eq!(bin.get_pixel(1, 1), true);
    }

    #[test]
    fn break_cluster_noop() {
        let image_string =
            "***\n".to_owned()+
            "***\n"+
            "-*-\n";
        let image = BinaryImage::from_string(&image_string);
        let clusters = Cluster::break_cluster(image.to_clusters(false).clusters.remove(0));
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters.get_cluster(0).to_binary_image().to_string(), image_string);
    }

    #[test]
    fn break_cluster() {
        let image = BinaryImage::from_string(&(
            "***---\n".to_owned()+
            "******\n"+
            "---***\n"));
        let clusters = Cluster::break_cluster(image.to_clusters(false).clusters.remove(0));
        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters.get_cluster(0).to_binary_image().to_string(),
            "***\n".to_owned()+
            "***\n");
        assert_eq!(clusters.get_cluster(0).rect.left, 0);
        assert_eq!(clusters.get_cluster(0).rect.top, 0);
        assert_eq!(clusters.get_cluster(1).to_binary_image().to_string(),
            "-**\n".to_owned()+
            "***\n");
        assert_eq!(clusters.get_cluster(1).rect.left, 3);
        assert_eq!(clusters.get_cluster(1).rect.top, 1);
    }

    #[test]
    fn break_cluster_alt() {
        let image = BinaryImage::from_string(&(
            "---***\n".to_owned()+
            "******\n"+
            "***---\n"));
        let clusters = Cluster::break_cluster(image.to_clusters(false).clusters.remove(0));
        for c in clusters.iter() {
            println!("{}", c.to_binary_image());
        }
        assert_eq!(clusters.len(), 2); 
        assert_eq!(clusters.get_cluster(0).to_binary_image().to_string(),
            "***\n".to_owned()+
            "-**\n");
        assert_eq!(clusters.get_cluster(0).rect.left, 3);
        assert_eq!(clusters.get_cluster(0).rect.top, 0);
        assert_eq!(clusters.get_cluster(1).to_binary_image().to_string(),
            "***\n".to_owned()+
            "***\n");
        assert_eq!(clusters.get_cluster(1).rect.left, 0);
        assert_eq!(clusters.get_cluster(1).rect.top, 1);
    }

    #[test]
    fn break_cluster_cant_break() {
        let image_string = 
            "*--\n".to_owned()+
            "**-\n"+
            "-**\n";
        let image = BinaryImage::from_string(&image_string);
        let clusters = Cluster::break_cluster(image.to_clusters(false).clusters.remove(0));
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters.get_cluster(0).to_binary_image().to_string(), image_string);
    }

    #[test]
    fn break_cluster_cant_break_2() {
        let image_string = 
            "*---\n".to_owned()+
            "****\n"+
            "--**\n";
        let image = BinaryImage::from_string(&image_string);
        let clusters = Cluster::break_cluster(image.to_clusters(false).clusters.remove(0));
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters.get_cluster(0).to_binary_image().to_string(), image_string);
    }

    #[test]
    fn break_cluster_big() {
        let image = BinaryImage::from_string(&(
            "***---***\n".to_owned()+
            "*********\n"+
            "---***---\n"));
        let clusters = Cluster::break_cluster(image.to_clusters(false).clusters.remove(0));
        assert_eq!(clusters.len(), 3);

        assert_eq!(clusters.get_cluster(0).to_binary_image().to_string(),
            "***\n".to_owned()+
            "***\n");
        assert_eq!(clusters.get_cluster(0).rect.left, 0);
        assert_eq!(clusters.get_cluster(0).rect.top, 0);

        assert_eq!(clusters.get_cluster(1).to_binary_image().to_string(),
            "***\n".to_owned()+
            "-**\n");
        assert_eq!(clusters.get_cluster(1).rect.left, 6);
        assert_eq!(clusters.get_cluster(1).rect.top, 0);

        assert_eq!(clusters.get_cluster(2).to_binary_image().to_string(),
            "-**\n".to_owned()+
            "***\n");
        assert_eq!(clusters.get_cluster(2).rect.left, 3);
        assert_eq!(clusters.get_cluster(2).rect.top, 1);
    }

    #[test]
    fn clusters_beyond_16_bit_labels() {
        // A checkerboard is the worst case for the scanline pass: with 4-connectivity
        // every set pixel is isolated, so each one takes a fresh label and none are
        // ever reclaimed by a merge. 512x512 needs 131072 of them.
        let size = 512;
        let mut image = BinaryImage::new_w_h(size, size);
        for y in 0..size {
            for x in 0..size {
                if (x + y) % 2 == 0 {
                    image.set_pixel(x, y, true);
                }
            }
        }
        let clusters = image.to_clusters(false);
        assert_eq!(clusters.len(), size * size / 2);
        assert!(clusters.clusters.iter().all(|c| c.size() == 1));
    }
}
