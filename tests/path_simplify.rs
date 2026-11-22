//! `PathSimplifyMode::Polygon` must not turn a pixel outline into a ring
//! that crosses itself or the cluster's other rings: the outlines feed
//! triangulation (`polypartition`) and filling, both of which assume
//! (weakly) simple polygons. Thin diagonal strokes are the hard case: a 1px
//! staircase cut short from one side lands on the opposite side 1-2px away.

use visioncortex::clusters::Cluster;
use visioncortex::{BinaryImage, PathI32, PathSimplifyMode, PointI32};

fn stroke(img: &mut BinaryImage, (x0, y0): (i32, i32), (dx, dy): (i32, i32), len: i32, width: i32) {
    let steps = dx.abs().max(dy.abs());
    for t in 0..=len * steps {
        let x = x0 + (t * dx).div_euclid(steps);
        let y = y0 + (t * dy).div_euclid(steps);
        for k in 0..width {
            let (px, py) = if dx.abs() >= dy.abs() { (x, y + k) } else { (x + k, y) };
            if px >= 0 && py >= 0 && (px as usize) < img.width && (py as usize) < img.height {
                img.set_pixel(px as usize, py as usize, true);
            }
        }
    }
}

/// Like an anti-aliased render thresholded at 50%: pixels whose centre lies
/// within `width / 2` of the segment. Float endpoints give the irregular
/// staircases real drawings have.
fn stroke_f(img: &mut BinaryImage, a: (f64, f64), b: (f64, f64), width: f64, value: bool) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    for y in 0..img.height {
        for x in 0..img.width {
            let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
            let t = if len2 > 0.0 { (((px - a.0) * dx + (py - a.1) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
            let (qx, qy) = (a.0 + t * dx - px, a.1 + t * dy - py);
            if (qx * qx + qy * qy).sqrt() < width / 2.0 {
                img.set_pixel(x, y, value);
            }
        }
    }
}

/// The cluster the render/triangulate fuzzer first caught crossing itself:
/// paper with a 1-2px ink crack running diagonally into it.
const CRACK: &str = "\
###########......
#############....
#############....
###############..
###..###########.
####..##########.
#####...#########
#######.....#####
....######.......
.....##########..
........######...";

fn from_ascii(art: &str) -> BinaryImage {
    let rows: Vec<&str> = art.lines().collect();
    let mut img = BinaryImage::new_w_h(rows[0].len(), rows.len());
    for (y, row) in rows.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            img.set_pixel(x, y, c == '#');
        }
    }
    img
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next() % (hi - lo + 1) as u64) as i32
    }
    fn f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (self.next() as f64 / (1u64 << 31) as f64) * (hi - lo)
    }
}

/// Thin strokes at many slopes and widths, alone, crossed, and in random
/// bundles.
fn corpus() -> Vec<BinaryImage> {
    let mut scenes = vec![];
    let slopes: [(i32, i32); 12] = [(1, 1), (2, 1), (1, 2), (3, 1), (3, 2), (5, 2), (7, 3), (1, -1), (2, -1), (5, -3), (4, 3), (9, 4)];
    for &(dx, dy) in &slopes {
        for width in 1..=3 {
            let mut img = BinaryImage::new_w_h(48, 48);
            let len = 40 / dx.abs().max(dy.abs());
            let start = if dy < 0 { (2, 44) } else { (2, 2) };
            stroke(&mut img, start, (dx, dy), len, width);
            scenes.push(img.clone());
            stroke(&mut img, (2, 24), (1, 0), 44, width);
            stroke(&mut img, (20, 2), (dy.abs().max(1), dx.abs().max(1)), 40 / dx.abs().max(dy.abs()).max(1), 2);
            scenes.push(img);
        }
    }
    let mut rng = Lcg(0x5EED);
    for _ in 0..300 {
        let mut img = BinaryImage::new_w_h(64, 64);
        for _ in 0..rng.range(1, 6) {
            let (dx, dy) = (rng.range(-9, 9), rng.range(-9, 9));
            if dx == 0 && dy == 0 {
                continue;
            }
            let steps = dx.abs().max(dy.abs());
            stroke(&mut img, (rng.range(0, 63), rng.range(0, 63)), (dx, dy), rng.range(2, 60) / steps + 1, rng.range(1, 3));
        }
        scenes.push(img);
    }
    // Rendered-looking strokes, ink on paper and paper cracks through ink.
    for i in 0..600 {
        let mut img = BinaryImage::new_w_h(40, 40);
        if i % 2 == 1 {
            stroke_f(&mut img, (rng.f64(0.0, 40.0), rng.f64(0.0, 40.0)), (rng.f64(0.0, 40.0), rng.f64(0.0, 40.0)), rng.f64(8.0, 30.0), true);
        }
        for _ in 0..rng.range(1, 4) {
            let a = (rng.f64(-2.0, 42.0), rng.f64(-2.0, 42.0));
            let b = (rng.f64(-2.0, 42.0), rng.f64(-2.0, 42.0));
            stroke_f(&mut img, a, b, rng.f64(1.0, 3.0), i % 2 == 0);
        }
        scenes.push(img);
    }
    scenes.push(from_ascii(CRACK));
    scenes
}

fn ring(path: &PathI32) -> Vec<PointI32> {
    let mut pts: Vec<PointI32> = vec![];
    for &p in path.path.iter() {
        if pts.last() != Some(&p) {
            pts.push(p);
        }
    }
    while pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    pts
}

fn orient(a: PointI32, b: PointI32, c: PointI32) -> i64 {
    (b.x - a.x) as i64 * (c.y - a.y) as i64 - (b.y - a.y) as i64 * (c.x - a.x) as i64
}

/// Pairs of edges, within or between rings, that cross properly or overlap
/// along a stretch of positive length. Touching at a point is allowed.
fn crossings(rings: &[Vec<PointI32>]) -> Vec<((PointI32, PointI32), (PointI32, PointI32))> {
    let edges: Vec<(PointI32, PointI32)> = rings
        .iter()
        .flat_map(|r| (0..r.len()).map(move |i| (r[i], r[(i + 1) % r.len()])))
        .collect();
    let mut out = vec![];
    for i in 0..edges.len() {
        for j in i + 1..edges.len() {
            let ((a, b), (c, d)) = (edges[i], edges[j]);
            let (o1, o2, o3, o4) = (orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b));
            let cross = (o1 > 0 && o2 < 0 || o1 < 0 && o2 > 0) && (o3 > 0 && o4 < 0 || o3 < 0 && o4 > 0);
            let overlap = o1 == 0 && o2 == 0 && {
                let along = |p: PointI32| (p.x - a.x) as i64 * (b.x - a.x) as i64 + (p.y - a.y) as i64 * (b.y - a.y) as i64;
                let ab = along(b);
                along(c).min(along(d)).max(0) < along(c).max(along(d)).min(ab)
            };
            if cross || overlap {
                out.push((edges[i], edges[j]));
            }
        }
    }
    out
}

/// Every 4-connected cluster of ink and of paper, as `image_to_paths` sees it.
fn clusters(img: &BinaryImage) -> Vec<BinaryImage> {
    [img.clone(), img.negative()]
        .iter()
        .flat_map(|layer| layer.to_clusters(false).iter().map(|c| c.to_binary_image()).collect::<Vec<_>>())
        .collect()
}

/// The pre-fix `limit_penalties`, unchanged, for comparison.
fn old_limit_penalties(path: &PathI32) -> PathI32 {
    let tolerance = 1.0;
    let path = &path.path;
    let len = path.len();
    let penalty = |a: PointI32, b: PointI32, c: PointI32| {
        let sq = |x: i32| (x * x) as f64;
        let l1 = (sq(a.x - b.x) + sq(a.y - b.y)).sqrt();
        let l2 = (sq(b.x - c.x) + sq(b.y - c.y)).sqrt();
        let l3 = (sq(c.x - a.x) + sq(c.y - a.y)).sqrt();
        let p = (l1 + l2 + l3) / 2.0;
        let area = (p * (p - l1) * (p - l2) * (p - l3)).sqrt();
        area * area / l3
    };
    let past_delta = |from: usize, to: usize| -> f64 {
        (from..to).skip(1).map(|i| penalty(path[from], path[i], path[to])).fold(0.0, |a, b| a.max(b))
    };
    let mut result = PathI32::new();
    if len == 0 {
        return result;
    }
    let mut last = 0;
    for i in 0..len {
        if i == 0 {
            result.add(path[i]);
        } else if i == last + 1 {
            continue;
        } else if past_delta(last, i) >= tolerance {
            last = i - 1;
            result.add(path[i - 1]);
        }
        if i == len - 1 {
            result.add(path[i]);
        }
    }
    result
}

/// The whole pre-fix Polygon-mode simplification of a cluster's rings.
fn old_simplify(base: &[PathI32]) -> Vec<PathI32> {
    base.iter().enumerate().map(|(i, p)| old_limit_penalties(&old_remove_staircase(p, i == 0))).collect()
}

/// Rings that keep at least three distinct points; the simplifier folds
/// rings of a few pixels down to a point or a sliver, as it always has.
fn solid_rings(paths: &[PathI32]) -> Vec<Vec<PointI32>> {
    paths.iter().map(ring).filter(|r| r.len() >= 3).collect()
}

#[test]
fn crack_outline_does_not_cross_itself() {
    let img = from_ascii(CRACK);
    for c in clusters(&img) {
        let rings = solid_rings(&Cluster::image_to_paths(&c, PathSimplifyMode::Polygon));
        assert_eq!(crossings(&rings), vec![], "rings {rings:?}");
    }
}

#[test]
fn simplified_rings_never_cross() {
    for (si, img) in corpus().iter().enumerate() {
        for c in clusters(img) {
            let rings = solid_rings(&Cluster::image_to_paths(&c, PathSimplifyMode::Polygon));
            let found = crossings(&rings);
            assert!(found.is_empty(), "scene {si}: crossing edges {found:?} in rings {rings:?}");
        }
    }
}

#[test]
fn tiny_rings_simplify_as_before() {
    // Rings of a few pixels keep exactly their old simplified form, which for
    // a lone pixel is no ring at all: spline tracing relies on that to drop
    // specks.
    let shapes = ["#", "##", "#\n#", "####", "##\n#.", "##\n##", ".#\n##\n#.", "#.\n##\n.#"];
    for art in shapes {
        let img = from_ascii(art);
        let new = Cluster::image_to_paths(&img, PathSimplifyMode::Polygon);
        let old = old_simplify(&Cluster::image_to_paths(&img, PathSimplifyMode::None));
        assert_eq!(
            new.iter().map(|p| p.path.clone()).collect::<Vec<_>>(),
            old.iter().map(|p| p.path.clone()).collect::<Vec<_>>(),
            "{art}"
        );
    }
    assert!(solid_rings(&Cluster::image_to_paths(&from_ascii("#"), PathSimplifyMode::Polygon)).is_empty());
}

#[test]
fn output_is_unchanged_wherever_it_was_already_simple() {
    // Only clusters whose old simplified rings crossed or overlapped change.
    let (mut same, mut changed) = (0, 0);
    for (si, img) in corpus().iter().enumerate() {
        for c in clusters(img) {
            let old = old_simplify(&Cluster::image_to_paths(&c, PathSimplifyMode::None));
            let new = Cluster::image_to_paths(&c, PathSimplifyMode::Polygon);
            let paths = |ps: &[PathI32]| ps.iter().map(|p| p.path.clone()).collect::<Vec<_>>();
            if crossings(&solid_rings(&old)).is_empty() {
                assert_eq!(paths(&new), paths(&old), "scene {si}");
                same += 1;
            } else {
                changed += 1;
            }
        }
    }
    assert!(changed > 0 && same > 100 * changed, "{same} unchanged, {changed} fixed");
}

/// The pre-fix `remove_staircase`, unchanged, for comparison.
fn old_remove_staircase(path: &PathI32, clockwise: bool) -> PathI32 {
    let path = &path.path;
    let len = path.len();
    let seg = |i: usize, j: usize| (path[i].x - path[j].x).abs() + (path[i].y - path[j].y).abs();
    let mut result = PathI32::new();
    for i in 0..len {
        let j = (i + 1) % len;
        let h = if i > 0 { i - 1 } else { len - 1 };
        let keep = if i == 0 || i == len - 1 {
            true
        } else if seg(i, h) == 1 || seg(i, j) == 1 {
            let area = orient(path[h], path[i], path[j]);
            area != 0 && (area > 0) == clockwise
        } else {
            true
        };
        if keep {
            result.add(path[i]);
        }
    }
    result
}
