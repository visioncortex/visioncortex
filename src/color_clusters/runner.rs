use crate::{Color, ColorImage, ColorI32};
use super::*;

pub struct Runner {
    config: RunnerConfig,
    image: ColorImage,
}

pub struct RunnerConfig {
    pub diagonal: bool,
    pub hierarchical: u32,
    pub batch_size: i32,
    pub good_min_area: usize,
    pub good_max_area: usize,
    pub is_same_color_a: i32,
    pub is_same_color_b: i32,
    pub deepen_diff: i32,
    pub hollow_neighbours: usize,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            diagonal: false,
            hierarchical: HIERARCHICAL_MAX,
            batch_size: 25600,
            good_min_area: 16,
            good_max_area: 256 * 256,
            is_same_color_a: 4,
            is_same_color_b: 1,
            deepen_diff: 64,
            hollow_neighbours: 1,
        }
    }
}

impl Default for Runner {
    fn default() -> Self {
        Self {
            config: RunnerConfig::default(),
            image: ColorImage::new(),
        }
    }
}

impl Runner {

    pub fn new(config: RunnerConfig, image: ColorImage) -> Self {
        Self {
            config,
            image
        }
    }

    pub fn init(&mut self, image: ColorImage) {
        self.image = image;
    }

    pub fn builder(self) -> Builder {
        let RunnerConfig {
            diagonal,
            hierarchical,
            batch_size,
            good_min_area,
            good_max_area,
            is_same_color_a,
            is_same_color_b,
            deepen_diff,
            hollow_neighbours,
        } = self.config;

        assert!(is_same_color_a < 8);

        Builder::new()
            .from(self.image)
            .diagonal(diagonal)
            .hierarchical(hierarchical)
            .batch_size(batch_size as u32)
            .same(move |a: Color, b: Color| {
                color_same(a, b, is_same_color_a, is_same_color_b)
            })
            .diff(color_diff)
            .deepen(move |parent: &ClustersView, patch: &Cluster, neighbours: &[NeighbourInfo]| {
                patch_good(parent, patch, good_min_area, good_max_area) &&
                neighbours[0].diff > deepen_diff
            })
            .hollow(move |_parent: &ClustersView, _patch: &Cluster, neighbours: &[NeighbourInfo]| {
                neighbours.len() <= hollow_neighbours
            })
    }

    pub fn start(self) -> IncrementalBuilder {
        self.builder().start()
    }

    pub fn run(self) -> Clusters {
        self.builder().run()
    }

}

pub fn color_diff(a: Color, b: Color) -> i32 {
    let a = ColorI32::new(&a);
    let b = ColorI32::new(&b);
    (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs()
}

pub fn color_same(a: Color, b: Color, shift: i32, thres: i32) -> bool {
    let diff = ColorI32 {
        r: (a.r >> shift) as i32,
        g: (a.g >> shift) as i32,
        b: (a.b >> shift) as i32,
    }
    .diff(&ColorI32 {
        r: (b.r >> shift) as i32,
        g: (b.g >> shift) as i32,
        b: (b.b >> shift) as i32,
    });

    diff.r.abs() <= thres && diff.g.abs() <= thres && diff.b.abs() <= thres
}

fn patch_good(
    parent: &ClustersView,
    patch: &Cluster,
    good_min_area: usize,
    good_max_area: usize
) -> bool {
    if good_min_area < patch.area() && patch.area() < good_max_area {
        if good_min_area == 0 ||
            (patch.perimeter(parent) as usize) < patch.area() {
            return true;
        } else {
            // cluster is thread-like and thinner than 2px
        }
    }
    false
}
