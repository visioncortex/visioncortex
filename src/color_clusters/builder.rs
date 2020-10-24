use crate::{Color, ColorImage};
use super::{Cluster, Clusters, ClustersView, container::ClusterIndex};

    #[derive(Clone)]
    pub struct BuilderConfig {
        pub(crate) diagonal: bool,
        pub(crate) batch_size: u32,
        pub(crate) key: Color,
    }

    impl Default for BuilderConfig {
        fn default() -> Self {
            Self {
                diagonal: true,
                batch_size: 10000,
                key: Color::default(),
            }
        }
    }

    pub struct NeighbourInfo {
        pub index: ClusterIndex,
        pub diff: i32,
    }

    type Cmp = Box<dyn Fn(Color, Color) -> bool>;
    type Diff = Box<dyn Fn(Color, Color) -> i32>;
    type Deepen = Box<dyn Fn(&ClustersView, &Cluster, &[NeighbourInfo]) -> bool>;
    type Hollow = Box<dyn Fn(&ClustersView, &Cluster, &[NeighbourInfo]) -> bool>;

    #[derive(Default)]
    pub struct Builder {
        pub(crate) conf: BuilderConfig,
        pub(crate) same: Option<Cmp>,
        pub(crate) diff: Option<Diff>,
        pub(crate) deepen: Option<Deepen>,
        pub(crate) hollow: Option<Hollow>,
        pub(crate) image: Option<ColorImage>,
    }

    pub struct IncrementalBuilder {
        builder_impl: Option<Box<BuilderImpl>>,
    }

    macro_rules! config_setter {
        ($name:ident, $t:ty) => {
            pub fn $name(mut self, $name: $t) -> Self {
                self.conf.$name = $name;
                self
            }
        };
    }

    macro_rules! closure_setter {
        ($name:ident, $t:path) => {
            pub fn $name(mut self, $name: impl $t + 'static) -> Self {
                self.$name = Some(Box::new($name));
                self
            }
        };
    }

    impl Builder {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn from(mut self, image: ColorImage) -> Self {
            self.image = Some(image);
            self
        }

        pub fn run(self) -> Clusters {
            let mut bimpl = BuilderImpl::from(self);
            while !bimpl.tick() {}
            bimpl.result()
        }

        pub fn start(self) -> IncrementalBuilder {
            IncrementalBuilder::new(BuilderImpl::from(self))
        }

        config_setter!(diagonal, bool);
        config_setter!(batch_size, u32);
        config_setter!(key, Color);

        closure_setter!(same, Fn(Color, Color) -> bool);
        closure_setter!(diff, Fn(Color, Color) -> i32);
        closure_setter!(deepen, Fn(&ClustersView, &Cluster, &[NeighbourInfo]) -> bool);
        closure_setter!(hollow, Fn(&ClustersView, &Cluster, &[NeighbourInfo]) -> bool);
    }

impl IncrementalBuilder {
    fn new(builder_impl: BuilderImpl) -> Self {
        Self {
            builder_impl: Some(Box::new(builder_impl))
        }
    }

    pub fn tick(&mut self) -> bool {
        self.builder_impl.as_mut().unwrap().tick()
    }

    pub fn result(&mut self) -> Clusters {
        self.builder_impl.take().unwrap().result()
	}

	pub fn progress(&self) -> u32 {
		match &self.builder_impl {
			None => {
				0
			},
			Some(builder) => {
				builder.as_ref().progress()
			}
		}
	}
}

struct Area {
    pub area: usize,
    pub count: usize,
}

struct BuilderImpl {
    diagonal: bool,
    batch_size: u32,
    key: Color,
    same: Cmp,
    diff: Diff,
    deepen: Deepen,
    hollow: Hollow,
    width: u32,
    height: u32,
    pixels: Vec<u8>,           // raw bytes from getImageData; 4 bytes as a pixel
    clusters: Vec<Cluster>,    // array of clusters
    cluster_indices: Vec<ClusterIndex>, // the cluster index each pixel belongs to
    cluster_areas: Vec<Area>,  // uniquely sorted array of cluster sizes
    clusters_output: Vec<ClusterIndex>, // indices of good clusters
    stage: u32,
    iteration: u32,
    next_index: ClusterIndex,
}

impl From<Builder> for BuilderImpl {
    // TODO: Provide default implementations.
    fn from(mut b: Builder) -> Self {
        let im = b.image.unwrap();
        let len = im.pixels.len();

        Self {
            diagonal: b.conf.diagonal,
            batch_size: b.conf.batch_size,
            key: b.conf.key,
            same: b.same.take().unwrap(),
            diff: b.diff.take().unwrap(),
            deepen: b.deepen.take().unwrap(),
            hollow: b.hollow.take().unwrap(),
            width: im.width as u32,
            height: im.height as u32,
            pixels: im.pixels,
            clusters: vec![Cluster::new()],
            cluster_indices: vec![Default::default(); len / 4],
            cluster_areas: Vec::new(),
            clusters_output: Vec::new(),
            stage: 1,
            iteration: 0,
            next_index: ClusterIndex(1),
        }
    }
}

impl BuilderImpl {
    pub fn tick(&mut self) -> bool {
        match self.stage {
            1 => {
                if self.stage_1() {
                    self.stage += 1;
                    self.iteration = 0;
                }
                false
            }
            2 => {
                for _i in 0..std::cmp::max(1, self.iteration / 16) {
                    if self.stage_2() {
                        self.stage += 1;
                        self.iteration = 0;
                        break;
                    }
                }
                false
            }
            _ => true,
        }
    }

    pub fn result(self) -> Clusters {
        Clusters {
            width: self.width,
            height: self.height,
            pixels: self.pixels,
            clusters: self.clusters,
            cluster_indices: self.cluster_indices,
            clusters_output: self.clusters_output,
        }
	}

	pub fn progress(&self) -> u32 {
		match self.stage {
			1 => {
				50 * self.iteration / self.cluster_indices.len() as u32
			},
			2 => {
				50 + 50 * self.iteration / self.cluster_areas.len() as u32
			},
			_ => {
				100
			}
		}
	}

    fn stage_1(&mut self) -> bool {
        let diagonal = self.diagonal;
        let batch_size = self.batch_size;
        let key = self.key;
        let has_key = key != Color::default();
        let len = self.cluster_indices.len();

        for i in (self.iteration..(self.iteration + batch_size)).take_while(|&i| (i as usize) < len)
        {
            let x = (i % self.width) as i32;
            let y = (i / self.width) as i32;

            let color = self.pixel_at(x, y);
            let up = self.pixel_at(x, y - 1);
            let left = self.pixel_at(x - 1, y);
            let upleft = self.pixel_at(x - 1, y - 1);

            let mut cluster_up = if y > 0 {
                self.cluster_indices[(self.width as i32 * (y - 1) + x) as usize].0
            } else {
                0
            } as usize;
            let mut cluster_left = if x > 0 {
                self.cluster_indices[(self.width as i32 * y + (x - 1)) as usize].0
            } else {
                0
            } as usize;
            let cluster_upleft = if x > 0 && y > 0 {
                self.cluster_indices[(self.width as i32 * (y - 1) + (x - 1)) as usize].0
            } else {
                0
            } as usize;

            if cluster_left != cluster_up
                && self.is_same(left, up)
                && (diagonal || // if not diagonal, self color must be same as up & left
                self.is_same(color, left) &&
                self.is_same(color, up))
            {
                if self.clusters[cluster_left].area() <= self.clusters[cluster_up].area() {
                    self.combine_clusters(ClusterIndex(cluster_left as u32), ClusterIndex(cluster_up as u32));
                    if cluster_left as u32 == self.next_index.0 - 1
                        && self.next_index.0 as usize == self.clusters.len()
                    {
                        // reduce cluster counts
                        self.next_index.0 -= 1;
                    }
                    cluster_left = cluster_up;
                } else {
                    self.combine_clusters(ClusterIndex(cluster_up as u32), ClusterIndex(cluster_left as u32));
                    cluster_up = cluster_left;
                }
            }

            let c = color.unwrap();

            if has_key && c == key {
                self.clusters[0].add(i, &c, x, y);
            } else if self.is_same(color, up) && self.is_same(color, upleft) {
                self.cluster_indices[i as usize] = ClusterIndex(cluster_up as u32);
                self.clusters[cluster_up].add(i, &c, x, y);
            } else if self.is_same(color, left) && self.is_same(color, upleft) {
                self.cluster_indices[i as usize] = ClusterIndex(cluster_left as u32);
                self.clusters[cluster_left].add(i, &c, x, y);
            } else if diagonal && self.is_same(color, upleft) {
                self.cluster_indices[i as usize] = ClusterIndex(cluster_upleft as u32);
                self.clusters[cluster_upleft].add(i, &c, x, y);
            } else {
                let mut new_cluster = Cluster::new();
                new_cluster.add(i, &c, x, y);
                if (self.next_index.0 as usize) < self.clusters.len() {
                    self.clusters[self.next_index.0 as usize] = new_cluster;
                } else {
                    self.clusters.push(new_cluster);
                }
                self.cluster_indices[i as usize] = self.next_index;
                self.next_index.0 += 1;
            }
        }

        self.iteration += batch_size;
        if self.iteration as usize >= self.cluster_indices.len() {
            self.initialize_areas_before_stage2();
            true
        } else {
            false
        }
    }

    fn stage_2(&mut self) -> bool {
        if self.iteration == 0 {
            for c in self.clusters.iter_mut() {
                c.residue = c.indices.clone();
                c.residue_sum = c.sum;
            }
        }

        if self.cluster_areas[self.iteration as usize].count == 0 {
            self.iteration += 1;

            if self.iteration as usize == self.cluster_areas.len() {
                return true;
            }

            return false;
        }

        let cur_area = self.cluster_areas[self.iteration as usize].area;

        for index in 0..self.clusters.len() {
            use std::collections::HashMap;

            if self.clusters[index].area() != cur_area {
                continue;
            }

            let mut votes = HashMap::new();

            for &i in self.clusters[index].iter() {
                let x = i % self.width;
                let y = i / self.width;

                for k in 0..4 {
                    let k = match k {
                        0 => if y > 0 { self.cluster_indices[(self.width * (y - 1) + x) as usize].0 } else { 0 },
                        1 => if y < self.height - 1 { self.cluster_indices[(self.width * (y + 1) + x) as usize].0 } else { 0 },
                        2 => if x > 0 { self.cluster_indices[(self.width * y + (x - 1)) as usize].0 } else { 0 },
                        3 => if x < self.width - 1 { self.cluster_indices[(self.width * y + (x + 1)) as usize].0 } else { 0 },
                        _ => unreachable!(),
                    };
                    if k > 0 && k as usize != index {
                        *votes.entry(k).or_insert(0) += 1;
                    }
                }
            }

            let color = self.clusters[index].color();
            let mut infos: Vec<_> = votes
                .iter()
                .filter(|(&k, _)| k > 0)
                .map(|(&k, _)| NeighbourInfo {
                    index: ClusterIndex(k),
                    diff: (self.diff)(color, self.clusters[k as usize].color()),
                })
                .collect();

            if infos.is_empty() {
                if self.iteration == self.cluster_areas.len() as u32 - 1 {
                    // this is the final background
                    self.clusters_output.push(ClusterIndex(index as u32));
                }
                continue;
            }

            infos.sort_by_key(|info| info.diff);

            let target = infos[0].index;

            let view = ClustersView {
                width: self.width,
                height: self.height,
                pixels: &self.pixels,
                clusters: &self.clusters,
                cluster_indices: &self.cluster_indices,
                clusters_output: &self.clusters_output,
            };

            let deepen = (self.deepen)(&view, &self.clusters[index], &infos);
            let hollow = (self.hollow)(&view, &self.clusters[index], &infos);

            if deepen {
                self.clusters_output.push(ClusterIndex(index as u32));
            }

            let target_in_areas = self
                .cluster_areas
                .binary_search_by_key(&self.clusters[target.0 as usize].area(), |a| a.area)
                .unwrap();

            self.cluster_areas[target_in_areas].count -= 1;
            self.merge_cluster_into(ClusterIndex(index as u32), target, deepen, hollow);
            let updated_area = self.clusters[target.0 as usize].area();

            match self
                .cluster_areas
                .binary_search_by_key(&updated_area, |a| a.area)
            {
                Ok(pos) => self.cluster_areas[pos].count += 1,
                Err(pos) => self.cluster_areas.insert(
                    pos,
                    Area {
                        area: updated_area,
                        count: 1,
                    },
                ),
            }
        }

        self.iteration += 1;
        self.iteration as usize == self.cluster_areas.len()
    }

    fn initialize_areas_before_stage2(&mut self) {
        use std::collections::HashMap;
        let mut counts = HashMap::new();

        for area in self
            .clusters
            .iter()
            .filter(|c| c.area() > 0)
            .map(|c| c.area())
        {
            *counts.entry(area).or_insert(0) += 1;
        }

        let mut areas = counts
            .into_iter()
            .map(|(k, v)| Area { area: k, count: v })
            .collect::<Vec<_>>();

        areas.sort_by_key(|a| a.area);

        self.cluster_areas = areas;
    }

    pub fn merge_cluster_into(&mut self, from: ClusterIndex, to: ClusterIndex, deepen: bool, hollow: bool) {
        if !deepen {
            let mut residue = self.clusters[from.0 as usize].residue.clone();
            let residue_sum = self.clusters[from.0 as usize].residue_sum;
            self.clusters[to.0 as usize].residue.append(&mut residue);
            self.clusters[to.0 as usize].residue_sum.merge(&residue_sum);
            self.combine_clusters(from, to);
        } else {
            self.combine_clusters_clone(from, to);

            if hollow {
                let mut holes = self.clusters[from.0 as usize].indices.clone();
                self.clusters[to.0 as usize].holes.append(&mut holes);
                self.clusters[to.0 as usize].num_holes += 1;
            }

            self.clusters[from.0 as usize].merged_into = Some(to);
            self.clusters[to.0 as usize].depth += 1;
        }
    }

    fn combine_clusters_clone(&mut self, from: ClusterIndex, to: ClusterIndex) {
        let sum = self.clusters[from.0 as usize].sum;
        let rect = self.clusters[from.0 as usize].rect;
        let indices = self.clusters[from.0 as usize].indices.clone();

        self.combine_clusters(from, to);

        self.clusters[from.0 as usize].sum = sum;
        self.clusters[from.0 as usize].rect = rect;
        self.clusters[from.0 as usize].indices = indices;
    }

    fn combine_clusters(&mut self, from: ClusterIndex, to: ClusterIndex) {
        for &i in self.clusters[from.0 as usize].indices.iter() {
            self.cluster_indices[i as usize] = to;
        }

        let mut indices = std::mem::replace(&mut self.clusters[from.0 as usize].indices, Vec::new());
        self.clusters[to.0 as usize].indices.append(&mut indices);
        let sum = self.clusters[from.0 as usize].sum;
        let rect = self.clusters[from.0 as usize].rect;
        self.clusters[to.0 as usize].sum.merge(&sum);
        self.clusters[to.0 as usize].rect.merge(rect);
        self.clusters[from.0 as usize].sum.clear();
        self.clusters[from.0 as usize].rect.clear();
    }

    fn is_same(&self, left: Option<Color>, right: Option<Color>) -> bool {
        if let (Some(l), Some(r)) = (left, right) {
            (self.same)(l, r)
        } else {
            false
        }
    }

    fn pixel_at(&self, x: i32, y: i32) -> Option<Color> {
        if x < 0 || y < 0 {
            return None;
        }

        self.get_pixel(y as u32 * self.width + x as u32)
    }

    fn get_pixel(&self, i: u32) -> Option<Color> {
        let i = i as usize * 4;
        if i < self.pixels.len() {
            Some(Color::new_rgba(
                self.pixels[i],
                self.pixels[i + 1],
                self.pixels[i + 2],
                self.pixels[i + 3],
            ))
        } else {
            None
        }
    }
}
