//! Round-trip fuzzer for `visioncortex::polypartition`.
//!
//! Each case, fully determined by its seed:
//!
//!  1. generate a random SVG scene (rects, ellipses, stars, self-intersecting
//!     polygons, rings, strokes, speckle, checkerboards, combs) in a small
//!     black and white only;
//!  2. render it with librsvg (anti-aliased) and threshold at 50% into a
//!     `BinaryImage`. Everything stays binary from here on;
//!  3. cluster the black pixels and the white pixels (4-connected);
//!  4. trace each cluster into an
//!     outer ring + hole rings (`PathSimplifyMode::None` = exact pixel
//!     staircase, `PathSimplifyMode::Polygon` = simplified);
//!  5. triangulate with ear clipping (after `remove_holes`), monotone, and
//!     optimal DP (hole-free inputs only);
//!  6. render the triangles again with librsvg (even-odd, so overlapping
//!     triangles cancel out instead of hiding), XOR against the reference
//!     (the cluster mask for `None`, the rendered polygon for `Polygon`),
//!     cluster the XOR, and flag the case if any diff blob exceeds `--tol`.
//!
//! Also checked: sum of triangle areas equals the polygon area (catches
//! overlaps and gaps below pixel resolution), and — for `None` only — that the
//! traced rings themselves reproduce the cluster mask (`sanity`, which blames
//! the tracer rather than polypartition).
//!
//! Panics are caught per stage, hangs are reported by a watchdog. For each
//! distinct failure signature the smallest repro is kept under `--out`.
//!
//! ```text
//! cargo run --release -- --iters 2000
//! cargo run --release -- --case 1234        # replay one seed verbosely
//! ```

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use visioncortex::clusters::Cluster as BinaryCluster;
use visioncortex::polypartition::{
    monotone_partition, remove_holes, triangulate_ec_vec, triangulate_mono, triangulate_mono_vec,
    triangulate_opt_vec, Orientation, Polygon, PolygonInterface,
};
use visioncortex::{BinaryImage, PathSimplifyMode, PointF64};

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Args {
    iters: u64,
    seed: u64,
    threads: usize,
    modes: Vec<Mode>,
    tol: usize,
    out: PathBuf,
    case: Option<u64>,
    render: Option<PathBuf>,
    input: Option<PathBuf>,
    thin: bool,
    timeout: Duration,
    max_size: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
enum Mode {
    None,
    Polygon,
}

impl Mode {
    fn simplify(self) -> PathSimplifyMode {
        match self {
            Mode::None => PathSimplifyMode::None,
            Mode::Polygon => PathSimplifyMode::Polygon,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Mode::None => "none",
            Mode::Polygon => "polygon",
        }
    }
}

const USAGE: &str = "\
polypartition-fuzz [options]
  --iters N      cases to run (0 = forever)            [1000]
  --seed S       first case seed                       [0]
  --threads T    worker threads                        [all cores]
  --mode M       none | polygon | both                 [both]
  --tol K        max allowed pixels in one diff blob   [0]
  --max-size P   max canvas side in pixels             [96]
  --thin         scenes of thin (1-3px) strokes only
  --timeout SEC  per-case hang threshold               [20]
  --out DIR      where repros are written              [out]
  --case S       run a single seed verbosely and save every finding
  --render FILE  rasterize an SVG with librsvg and print it as ASCII (debug)
  --input FILE   triangulate a saved input.txt with every algorithm (debug)";

fn parse_args() -> Args {
    let mut a = Args {
        iters: 1000,
        seed: 0,
        threads: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4),
        modes: vec![Mode::None, Mode::Polygon],
        tol: 0,
        out: PathBuf::from("out"),
        case: None,
        render: None,
        input: None,
        thin: false,
        timeout: Duration::from_secs(20),
        max_size: 96,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut val = || {
            it.next().unwrap_or_else(|| {
                eprintln!("missing value for {flag}\n\n{USAGE}");
                std::process::exit(2)
            })
        };
        let num = |s: String| -> u64 {
            s.parse().unwrap_or_else(|_| {
                eprintln!("not a number: {s}");
                std::process::exit(2)
            })
        };
        match flag.as_str() {
            "--iters" => a.iters = num(val()),
            "--seed" => a.seed = num(val()),
            "--threads" => a.threads = num(val()).max(1) as usize,
            "--tol" => a.tol = num(val()) as usize,
            "--max-size" => a.max_size = num(val()).clamp(8, 4096) as u32,
            "--timeout" => a.timeout = Duration::from_secs(num(val())),
            "--out" => a.out = PathBuf::from(val()),
            "--case" => a.case = Some(num(val())),
            "--render" => a.render = Some(PathBuf::from(val())),
            "--input" => a.input = Some(PathBuf::from(val())),
            "--thin" => a.thin = true,
            "--mode" => {
                a.modes = match val().as_str() {
                    "none" => vec![Mode::None],
                    "polygon" => vec![Mode::Polygon],
                    "both" => vec![Mode::None, Mode::Polygon],
                    m => {
                        eprintln!("unknown mode {m}\n\n{USAGE}");
                        std::process::exit(2)
                    }
                }
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0)
            }
            f => {
                eprintln!("unknown flag {f}\n\n{USAGE}");
                std::process::exit(2)
            }
        }
    }
    a
}

// ---------------------------------------------------------------------------
// RNG (splitmix64): reproducible from the case seed, no extra deps
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03)
    }
    fn u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn f64(&mut self) -> f64 {
        (self.u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Inclusive range.
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.u64() % ((hi - lo + 1) as u64)) as i64
    }
    fn chance(&mut self, p: f64) -> bool {
        self.f64() < p
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.range(0, xs.len() as i64 - 1) as usize]
    }
}

// ---------------------------------------------------------------------------
// 1. Random scene
// ---------------------------------------------------------------------------

struct Scene {
    w: u32,
    h: u32,
    svg: String,
}

/// A coordinate on one of three grids: integer, half-pixel, or free.
/// Integer-aligned shapes give long axis-parallel runs; free ones give
/// jagged staircases.
fn coord(rng: &mut Rng, grid: u8, lo: f64, hi: f64) -> f64 {
    let v = lo + rng.f64() * (hi - lo);
    match grid {
        0 => v.round(),
        1 => (v * 2.0).round() / 2.0,
        _ => (v * 1000.0).round() / 1000.0,
    }
}

fn gen_scene(seed: u64, max_size: u32, thin: bool) -> Scene {
    let mut rng = Rng::new(seed);
    let w = rng.range(8, max_size as i64) as u32;
    let h = rng.range(8, max_size as i64) as u32;
    let (fw, fh) = (w as f64, h as f64);

    // Black ink on white paper; white shapes carve holes and split blobs.
    let mut body = String::new();
    let _ = write!(body, r#"<rect width="{w}" height="{h}" fill="white"/>"#);

    let nshapes = rng.range(1, 14);
    for _ in 0..nshapes {
        let fill = if rng.chance(0.7) { "black" } else { "white" };
        let grid = rng.range(0, 2) as u8;
        match if thin { 9 } else { rng.range(0, 10) } {
            0 => {
                let x = coord(&mut rng, grid, -4.0, fw);
                let y = coord(&mut rng, grid, -4.0, fh);
                let rw = coord(&mut rng, grid, 1.0, fw * 0.7);
                let rh = coord(&mut rng, grid, 1.0, fh * 0.7);
                let _ = write!(body, r#"<rect x="{x}" y="{y}" width="{rw}" height="{rh}" fill="{fill}"/>"#);
            }
            1 => {
                let cx = coord(&mut rng, grid, 0.0, fw);
                let cy = coord(&mut rng, grid, 0.0, fh);
                let rw = coord(&mut rng, grid, 2.0, fw * 0.6);
                let rh = coord(&mut rng, grid, 2.0, fh * 0.6);
                let a = rng.range(0, 359);
                let _ = write!(
                    body,
                    r#"<rect x="{}" y="{}" width="{rw}" height="{rh}" transform="rotate({a} {cx} {cy})" fill="{fill}"/>"#,
                    cx - rw / 2.0,
                    cy - rh / 2.0
                );
            }
            2 => {
                let cx = coord(&mut rng, grid, 0.0, fw);
                let cy = coord(&mut rng, grid, 0.0, fh);
                let rx = coord(&mut rng, grid, 0.5, fw * 0.5);
                let ry = coord(&mut rng, grid, 0.5, fh * 0.5);
                let _ = write!(body, r#"<ellipse cx="{cx}" cy="{cy}" rx="{rx}" ry="{ry}" fill="{fill}"/>"#);
            }
            3 => {
                // star
                let n = rng.range(3, 12);
                let cx = coord(&mut rng, grid, 0.0, fw);
                let cy = coord(&mut rng, grid, 0.0, fh);
                let ro = 2.0 + rng.f64() * fw.min(fh) * 0.5;
                let ri = ro * (0.15 + rng.f64() * 0.7);
                let rot = rng.f64() * std::f64::consts::TAU;
                let mut pts = String::new();
                for k in 0..2 * n {
                    let r = if k % 2 == 0 { ro } else { ri };
                    let t = rot + k as f64 * std::f64::consts::PI / n as f64;
                    let _ = write!(pts, "{:.3},{:.3} ", cx + r * t.cos(), cy + r * t.sin());
                }
                let _ = write!(body, r#"<polygon points="{pts}" fill="{fill}"/>"#);
            }
            4 => {
                // random, usually self-intersecting polygon
                let n = rng.range(3, 10);
                let rule = if rng.chance(0.5) { "evenodd" } else { "nonzero" };
                let mut pts = String::new();
                for _ in 0..n {
                    let _ = write!(
                        pts,
                        "{},{} ",
                        coord(&mut rng, grid, -2.0, fw + 2.0),
                        coord(&mut rng, grid, -2.0, fh + 2.0)
                    );
                }
                let _ = write!(body, r#"<polygon points="{pts}" fill-rule="{rule}" fill="{fill}"/>"#);
            }
            5 => {
                // ring(s): concentric boxes or ellipses with holes, sometimes an island
                let cx = coord(&mut rng, grid, 0.0, fw);
                let cy = coord(&mut rng, grid, 0.0, fh);
                let r0 = 3.0 + rng.f64() * fw.min(fh) * 0.5;
                let rings = rng.range(2, 4);
                let square = rng.chance(0.5);
                let mut d = String::new();
                for k in 0..rings {
                    let r = r0 * (1.0 - k as f64 / (rings as f64 + 0.5));
                    let r = if grid == 0 { r.round().max(1.0) } else { r };
                    if square {
                        let _ = write!(d, "M{} {}h{}v{}h{}Z", cx - r, cy - r, 2.0 * r, 2.0 * r, -2.0 * r);
                    } else {
                        let _ = write!(d, "M{} {}a{r} {r} 0 1 0 {} 0a{r} {r} 0 1 0 {} 0Z", cx - r, cy, 2.0 * r, -2.0 * r);
                    }
                }
                let _ = write!(body, r#"<path d="{d}" fill-rule="evenodd" fill="{fill}"/>"#);
            }
            6 => {
                // thin stroked polyline: 1px features, diagonal pinches
                let n = rng.range(2, 6);
                let sw = *rng.pick(&[0.5, 1.0, 1.0, 1.5, 2.0, 3.0]);
                let mut pts = String::new();
                for _ in 0..n {
                    let _ = write!(pts, "{},{} ", coord(&mut rng, grid, 0.0, fw), coord(&mut rng, grid, 0.0, fh));
                }
                let _ = write!(
                    body,
                    r#"<polyline points="{pts}" fill="none" stroke="{fill}" stroke-width="{sw}"/>"#
                );
            }
            7 => {
                // speckle: isolated pixels and tiny blobs
                let n = rng.range(1, 40);
                for _ in 0..n {
                    let x = rng.range(0, w as i64 - 1);
                    let y = rng.range(0, h as i64 - 1);
                    let s = rng.range(1, 2);
                    let _ = write!(body, r#"<rect x="{x}" y="{y}" width="{s}" height="{s}" fill="{fill}"/>"#);
                }
            }
            8 => {
                // checkerboard patch: lots of diagonal-only contacts
                let x0 = rng.range(0, w as i64 - 1);
                let y0 = rng.range(0, h as i64 - 1);
                let cw = rng.range(2, 12);
                let ch = rng.range(2, 12);
                let cell = rng.range(1, 3);
                let mut d = String::new();
                for j in 0..ch {
                    for i in 0..cw {
                        if (i + j) % 2 == 0 && rng.chance(0.9) {
                            let _ = write!(d, "M{} {}h{cell}v{cell}h-{cell}Z", x0 + i * cell, y0 + j * cell);
                        }
                    }
                }
                let _ = write!(body, r#"<path d="{d}" fill="{fill}"/>"#);
            }
            9 => {
                // thin strokes, ~2px wide: long diagonal staircases, near
                // misses and crossings, the hardest shapes for the tracer's
                // outlines to triangulate
                let n = rng.range(1, 6);
                let cap = *rng.pick(&["butt", "round", "square"]);
                let join = *rng.pick(&["miter", "round", "bevel"]);
                let mut d = String::new();
                for _ in 0..n {
                    let (x0, y0) = (coord(&mut rng, grid, 0.0, fw), coord(&mut rng, grid, 0.0, fh));
                    let _ = write!(d, "M{x0} {y0}");
                    match rng.range(0, 3) {
                        0 => {
                            let _ = write!(d, "L{} {}", coord(&mut rng, grid, 0.0, fw), coord(&mut rng, grid, 0.0, fh));
                        }
                        1 => {
                            for _ in 0..rng.range(2, 8) {
                                let _ = write!(d, "L{} {}", coord(&mut rng, grid, 0.0, fw), coord(&mut rng, grid, 0.0, fh));
                            }
                        }
                        2 => {
                            let _ = write!(
                                d,
                                "Q{} {} {} {}",
                                coord(&mut rng, grid, 0.0, fw),
                                coord(&mut rng, grid, 0.0, fh),
                                coord(&mut rng, grid, 0.0, fw),
                                coord(&mut rng, grid, 0.0, fh)
                            );
                        }
                        _ => {
                            let r = coord(&mut rng, grid, 2.0, fw.min(fh) * 0.4);
                            let _ = write!(d, "m-{r} 0a{r} {r} 0 1 0 {} 0a{r} {r} 0 1 0 -{} 0", 2.0 * r, 2.0 * r);
                        }
                    }
                }
                let sw = *rng.pick(&[1.0, 1.5, 2.0, 2.0, 2.0, 2.5, 3.0]);
                let _ = write!(
                    body,
                    r#"<path d="{d}" fill="none" stroke="{fill}" stroke-width="{sw}" stroke-linecap="{cap}" stroke-linejoin="{join}"/>"#
                );
            }
            _ => {
                // comb: a bar with teeth of random width/length on one side
                let x0 = rng.range(0, w as i64 / 2);
                let y0 = rng.range(0, h as i64 - 3);
                let len = rng.range(4, (w as i64 - x0).max(4));
                let mut d = format!("M{x0} {y0}h{len}v2");
                let mut x = x0 + len;
                while x > x0 {
                    let tw = rng.range(1, 3).min(x - x0);
                    let tl = rng.range(0, 8);
                    let _ = write!(d, "h-{tw}v{tl}");
                    x -= tw;
                    let gw = rng.range(1, 3).min(x - x0);
                    let _ = write!(d, "v-{tl}h-{gw}");
                    x -= gw;
                }
                let _ = write!(d, "Z");
                let _ = write!(body, r#"<path d="{d}" fill="{fill}"/>"#);
            }
        }
    }

    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">{body}</svg>"#
    );

    Scene { w, h, svg }
}

// ---------------------------------------------------------------------------
// 2. librsvg rendering
// ---------------------------------------------------------------------------

/// Renders an SVG at 1 unit = 1 px and returns (w, h, BGRA bytes, stride).
fn rsvg_render(svg: &str, w: u32, h: u32) -> Result<(Vec<u8>, usize), String> {
    let bytes = glib::Bytes::from(svg.as_bytes());
    let stream = gio::MemoryInputStream::from_bytes(&bytes);
    let handle = rsvg::Loader::new()
        .read_stream(&stream, None::<&gio::File>, None::<&gio::Cancellable>)
        .map_err(|e| format!("rsvg load: {e}"))?;
    let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, w as i32, h as i32)
        .map_err(|e| format!("cairo surface: {e}"))?;
    {
        let cr = cairo::Context::new(&surface).map_err(|e| format!("cairo ctx: {e}"))?;
        rsvg::CairoRenderer::new(&handle)
            .render_document(&cr, &cairo::Rectangle::new(0.0, 0.0, w as f64, h as f64))
            .map_err(|e| format!("rsvg render: {e}"))?;
    }
    surface.flush();
    let stride = surface.stride() as usize;
    let data = surface.data().map_err(|e| format!("cairo data: {e}"))?.to_vec();
    Ok((data, stride))
}

/// The scene is opaque black-on-white: ink = darker than 50% grey.
fn render_scene(svg: &str, w: u32, h: u32) -> Result<BinaryImage, String> {
    let (data, stride) = rsvg_render(svg, w, h)?;
    let mut img = BinaryImage::new_w_h(w as usize, h as usize);
    for y in 0..h as usize {
        for x in 0..w as usize {
            // Cairo ARGB32 is native-endian u32; opaque, so red is the grey level.
            let px = u32::from_ne_bytes(data[y * stride + x * 4..][..4].try_into().unwrap());
            img.set_pixel(x, y, ((px >> 16) & 0xff) < 128);
        }
    }
    Ok(img)
}

/// Anti-aliased coverage thresholded at 50%. Deliberately not
/// `shape-rendering="crispEdges"`: cairo's non-AA rasterizer fills the gap
/// between disjoint subpaths of one path (two triangles 1px apart render as
/// one solid block), which would be blamed on the triangulation.
///
/// Returns (mask, unsure): `unsure` marks pixels whose coverage is within
/// `TIE_BAND` of 50%. Those are pixel centres sitting (almost) exactly on an
/// edge, and whether they land at 127 or 128 depends on what other edges share
/// the scanline, so an exact triangulation can still flip them. They are left
/// out of the XOR; the exact area check still catches overlaps and gaps there.
fn render_mask(svg: &str, w: u32, h: u32) -> Result<(BinaryImage, BinaryImage), String> {
    let (data, stride) = rsvg_render(svg, w, h)?;
    let mut img = BinaryImage::new_w_h(w as usize, h as usize);
    let mut unsure = BinaryImage::new_w_h(w as usize, h as usize);
    for y in 0..h as usize {
        for x in 0..w as usize {
            let px = u32::from_ne_bytes(data[y * stride + x * 4..][..4].try_into().unwrap());
            let a = (px >> 24) as i32;
            img.set_pixel(x, y, a >= 128);
            unsure.set_pixel(x, y, (a - 128).abs() <= TIE_BAND);
        }
    }
    Ok((img, unsure))
}

/// Cairo's AA rasterizer samples ~15 sub-scanlines per pixel, so a tie can be
/// off by up to ~1/15 of full coverage.
const TIE_BAND: i32 = 24;

/// Pixel padding around a component so simplified rings that stray past the
/// bbox are still rendered.
const PAD: i32 = 2;

fn rings_svg(rings: &[Vec<PointF64>], w: u32, h: u32) -> String {
    let mut d = String::new();
    for ring in rings {
        for (i, p) in ring.iter().enumerate() {
            let _ = write!(d, "{}{} {}", if i == 0 { 'M' } else { 'L' }, p.x + PAD as f64, p.y + PAD as f64);
        }
        d.push('Z');
    }
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}"><path fill-rule="evenodd" d="{d}"/></svg>"#
    )
}

// ---------------------------------------------------------------------------
// 3-4. Cluster, trace
// ---------------------------------------------------------------------------

struct Component {
    /// Black or white pixels, and which 4-connected cluster of them.
    cluster: &'static str,
    piece: usize,
    /// Mask in component-local coords, padded by PAD on every side.
    mask: BinaryImage,
    /// Offset of the unpadded component in the scene.
    origin: (i32, i32),
}

fn components(image: &BinaryImage) -> Vec<Component> {
    let mut out = vec![];
    for (name, layer) in [("black", image.clone()), ("white", image.negative())] {
        for (pi, piece) in layer.to_clusters(false).iter().enumerate() {
            let w = piece.rect.width() as usize + 2 * PAD as usize;
            let h = piece.rect.height() as usize + 2 * PAD as usize;
            let mut mask = BinaryImage::new_w_h(w, h);
            for p in piece.points.iter() {
                mask.set_pixel(
                    (p.x - piece.rect.left + PAD) as usize,
                    (p.y - piece.rect.top + PAD) as usize,
                    true,
                );
            }
            out.push(Component { cluster: name, piece: pi, mask, origin: (piece.rect.left, piece.rect.top) });
        }
    }
    out
}

/// Outer ring first (CCW per polypartition's convention), then holes (CW).
/// Coordinates are relative to the unpadded component.
fn trace(comp: &Component, mode: Mode) -> Vec<Polygon> {
    // image_to_paths expects the tight image; strip the padding.
    let (w, h) = (comp.mask.width - 2 * PAD as usize, comp.mask.height - 2 * PAD as usize);
    let mut tight = BinaryImage::new_w_h(w, h);
    for y in 0..h {
        for x in 0..w {
            tight.set_pixel(x, y, comp.mask.get_pixel(x + PAD as usize, y + PAD as usize));
        }
    }
    let paths = BinaryCluster::image_to_paths(&tight, mode.simplify());
    let mut polys = vec![];
    for (i, path) in paths.iter().enumerate() {
        let mut pts: Vec<PointF64> = Vec::with_capacity(path.path.len());
        for p in path.path.iter() {
            let q = PointF64::new(p.x as f64, p.y as f64);
            if pts.last() != Some(&q) {
                pts.push(q);
            }
        }
        while pts.len() > 1 && pts.first() == pts.last() {
            pts.pop();
        }
        let is_hole = i > 0;
        let mut poly = Polygon::from_points_and_is_hole(pts, is_hole);
        poly.props_mut().set_orientation(if is_hole {
            Orientation::Clockwise
        } else {
            Orientation::CounterClockwise
        });
        polys.push(poly);
    }
    polys
}

/// Parses the `input.txt` dump format: polygon count, then per polygon its
/// point count, hole flag (0/1) and one "x y" line per point.
fn parse_input(text: &str) -> Vec<Polygon> {
    let mut tok = text.split_whitespace().map(|t| t.parse::<f64>().expect("number"));
    let mut next = || tok.next().expect("truncated input");
    let count = next() as usize;
    (0..count)
        .map(|_| {
            let n = next() as usize;
            let hole = next() != 0.0;
            let pts = (0..n).map(|_| { let x = next(); PointF64::new(x, next()) }).collect();
            Polygon::from_points_and_is_hole(pts, hole)
        })
        .collect()
}

fn replay_input(text: &str) {
    let polys = parse_input(text);
    let want: f64 = polys.iter().map(|p| signed_area(&ring_points(p)).abs() * if p.is_hole() { -1.0 } else { 1.0 }).sum();
    println!("{} polygons, area {want}, weakly simple: {}", polys.len(), is_weakly_simple(&polys));
    match catch(|| remove_holes(&polys).map(|v| v.to_vec())) {
        Ok(Ok(merged)) => {
            for m in &merged {
                let pts = &m.props().points;
                let n = pts.len();
                let o = |a: PointF64, b: PointF64, c: PointF64| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
                for i in 0..n {
                    for j in i + 1..n {
                        let (a, b, c, d) = (pts[i], pts[(i + 1) % n], pts[j], pts[(j + 1) % n]);
                        if o(a, b, c) * o(a, b, d) < 0.0 && o(c, d, a) * o(c, d, b) < 0.0 {
                            println!("merged ring crosses: {a:?}-{b:?} x {c:?}-{d:?}");
                        }
                    }
                }
                println!("merged ring area {}", signed_area(pts));
                let pts: Vec<String> = m.props().points.iter().map(|p| format!("{} {}", p.x, p.y)).collect();
                println!("merged: {}", pts.join(", "));
            }
        }
        Ok(Err(e)) => println!("remove_holes: {e}"),
        Err(e) => println!("remove_holes PANIC: {e}"),
    }
    for algo in [Algo::Ec, Algo::Mono, Algo::Opt] {
        if algo == Algo::Opt && polys.iter().any(|p| p.is_hole()) {
            continue;
        }
        let t0 = Instant::now();
        let result = catch(|| triangulate(algo, &polys));
        let elapsed = t0.elapsed();
        match result {
            Ok(Ok(tris)) => {
                let got: f64 = tris.iter().map(|t| signed_area(&ring_points(t)).abs()).sum();
                let flag = if got == want { "ok" } else { "AREA MISMATCH" };
                println!("{:4}: {} triangles, area {got} {flag} in {:.1?}", algo.name(), tris.len(), elapsed);
                // Off-lattice coverage samples: exactly one triangle inside, none outside.
                let rings: Vec<Vec<PointF64>> = polys.iter().map(ring_points).collect();
                let inside_ring = |r: &[PointF64], x: f64, y: f64| {
                    let mut c = false;
                    for i in 0..r.len() {
                        let (a, b) = (r[i], r[(i + 1) % r.len()]);
                        if (a.y > y) != (b.y > y) && x < a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y) {
                            c = !c;
                        }
                    }
                    c
                };
                let in_tri = |t: &Polygon, x: f64, y: f64| {
                    let p = t.props().points.clone();
                    let s = |a: PointF64, b: PointF64| (b.x - a.x) * (y - a.y) - (b.y - a.y) * (x - a.x);
                    let (s1, s2, s3) = (s(p[0], p[1]), s(p[1], p[2]), s(p[2], p[0]));
                    (s1 > 0.0 && s2 > 0.0 && s3 > 0.0) || (s1 < 0.0 && s2 < 0.0 && s3 < 0.0)
                };
                let maxx = rings.iter().flatten().map(|p| p.x).fold(0.0, f64::max);
                let maxy = rings.iter().flatten().map(|p| p.y).fold(0.0, f64::max);
                let mut bad = 0;
                let mut y = 0.612847;
                while y < maxy {
                    let mut x = 0.371932;
                    while x < maxx {
                        let inside = rings.iter().filter(|r| inside_ring(r, x, y)).count() % 2 == 1;
                        let cover: Vec<&Polygon> = tris.iter().filter(|t| in_tri(t, x, y)).collect();
                        if cover.len() != inside as usize {
                            if bad < 5 {
                                println!("      coverage {} at ({x}, {y}), inside {inside}", cover.len());
                                for t in cover {
                                    let pts: Vec<String> = t.props().points.iter().map(|p| format!("{} {}", p.x, p.y)).collect();
                                    println!("        by {}", pts.join(", "));
                                }
                            }
                            bad += 1;
                        }
                        x += 1.0;
                    }
                    y += 1.0;
                }
                if bad > 0 {
                    println!("      COVERAGE MISMATCH at {bad} samples");
                }
                if got != want {
                    for t in &tris {
                        let pts: Vec<String> = t.props().points.iter().map(|p| format!("{} {}", p.x, p.y)).collect();
                        println!("      {}", pts.join(", "));
                    }
                }
            }
            Ok(Err(e)) => {
                println!("{:4}: error: {e}", algo.name());
                if algo == Algo::Mono {
                    if let Ok(Ok(pieces)) = catch(|| monotone_partition(polys.clone())) {
                        for piece in &pieces {
                            let pts: Vec<String> = piece.props().points.iter().map(|p| format!("{} {}", p.x, p.y)).collect();
                            let ok = match triangulate_mono(piece) { Ok(_) => "ok".to_string(), Err(e) => e.to_string() };
                            println!("      piece [{ok}]: {}", pts.join(", "));
                        }
                    }
                }
            }
            Err(e) => println!("{:4}: PANIC {e}", algo.name()),
        }
    }
}

fn ring_points(p: &Polygon) -> Vec<PointF64> {
    p.props().points.clone()
}

fn signed_area(pts: &[PointF64]) -> f64 {
    let n = pts.len();
    let mut a = 0.0;
    for i in 0..n {
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        a += p.x * q.y - q.x * p.y;
    }
    a / 2.0
}

/// True if no two edges cross properly (touching at shared vertices, as the
/// pixel tracer produces at diagonal pinches, is allowed).
fn is_weakly_simple(polys: &[Polygon]) -> bool {
    let mut edges = vec![];
    for p in polys {
        let pts = &p.props().points;
        for i in 0..pts.len() {
            edges.push((pts[i], pts[(i + 1) % pts.len()]));
        }
    }
    let orient = |a: PointF64, b: PointF64, c: PointF64| -> f64 { (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x) };
    for i in 0..edges.len() {
        for j in i + 1..edges.len() {
            let (a, b) = edges[i];
            let (c, d) = edges[j];
            let (o1, o2) = (orient(a, b, c), orient(a, b, d));
            let (o3, o4) = (orient(c, d, a), orient(c, d, b));
            if o1 * o2 < 0.0 && o3 * o4 < 0.0 {
                return false;
            }
            // Collinear overlap of positive length also breaks simplicity.
            if o1 == 0.0 && o2 == 0.0 {
                let (t0, t1) = if (b.x - a.x).abs() >= (b.y - a.y).abs() {
                    let (lo, hi) = (a.x.min(b.x), a.x.max(b.x));
                    (c.x.max(lo).min(hi), d.x.max(lo).min(hi))
                } else {
                    let (lo, hi) = (a.y.min(b.y), a.y.max(b.y));
                    (c.y.max(lo).min(hi), d.y.max(lo).min(hi))
                };
                if (t0 - t1).abs() > 0.0 && !(i + 1 == j || (j + 1) % edges.len() == i) {
                    // shared-segment between non-adjacent edges
                    let len = |p: PointF64, q: PointF64| ((p.x - q.x).powi(2) + (p.y - q.y).powi(2)).sqrt();
                    if len(a, b) > 0.0 && len(c, d) > 0.0 {
                        return false;
                    }
                }
            }
        }
    }
    true
}

// ---------------------------------------------------------------------------
// 5-6. Triangulate, compare
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
enum Algo {
    Ec,
    Mono,
    Opt,
}

impl Algo {
    fn name(self) -> &'static str {
        match self {
            Algo::Ec => "ec",
            Algo::Mono => "mono",
            Algo::Opt => "opt",
        }
    }
}

fn triangulate(algo: Algo, polys: &[Polygon]) -> Result<Vec<Polygon>, String> {
    match algo {
        Algo::Ec => {
            let merged = remove_holes(polys).map_err(|e| format!("remove_holes: {e}"))?;
            triangulate_ec_vec(merged)
        }
        Algo::Mono => triangulate_mono_vec(polys.to_vec()),
        Algo::Opt => triangulate_opt_vec(polys.to_vec()),
    }
}

thread_local! {
    static LAST_PANIC: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn install_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string payload>".into()
        };
        let loc = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        LAST_PANIC.with(|p| *p.borrow_mut() = Some(format!("{msg} @ {loc}")));
    }));
}

fn catch<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    panic::catch_unwind(AssertUnwindSafe(f))
        .map_err(|_| LAST_PANIC.with(|p| p.borrow_mut().take()).unwrap_or_else(|| "panic".into()))
}

#[derive(Clone, Debug)]
enum Kind {
    /// Panic outside polypartition (render / cluster / trace).
    HarnessPanic(&'static str, String),
    /// Traced rings don't reproduce the cluster mask (tracer, not polypartition).
    Sanity(usize),
    Panic(String),
    Error(String),
    Malformed(String),
    Area { want: f64, got: f64 },
    Diverged { blob: usize, total: usize },
    Hang(&'static str),
}

#[derive(Clone)]
struct Finding {
    seed: u64,
    mode: Option<Mode>,
    algo: Option<Algo>,
    kind: Kind,
    simple: bool,
    points: usize,
    holes: usize,
    /// Repro material: input dump, triangles svg, diff image.
    polys: Vec<Polygon>,
    tris: Vec<Polygon>,
    reference: Option<BinaryImage>,
    got: Option<BinaryImage>,
    /// The traced cluster (padded by PAD), to reproduce the tracing itself.
    mask: Option<BinaryImage>,
    where_: String,
}

impl Finding {
    fn signature(&self) -> String {
        let scope = format!(
            "{}{}",
            self.mode.map(|m| m.name()).unwrap_or("-"),
            self.algo.map(|a| format!("/{}", a.name())).unwrap_or_default()
        );
        let tag = if self.simple { "" } else { " [non-simple input]" };
        let k = match &self.kind {
            Kind::HarnessPanic(stage, m) => format!("harness panic in {stage}: {}", strip_msg(m)),
            Kind::Sanity(_) => "sanity: traced rings != cluster mask".into(),
            Kind::Panic(m) => format!("panic: {}", strip_msg(m)),
            Kind::Error(m) => format!("error: {m}"),
            Kind::Malformed(m) => format!("malformed output: {m}"),
            Kind::Area { .. } => "area mismatch".into(),
            Kind::Diverged { .. } => "diverged".into(),
            Kind::Hang(stage) => format!("hang in {stage}"),
        };
        format!("{scope:<12} {k}{tag}")
    }
    fn short(&self) -> String {
        match &self.kind {
            Kind::Area { want, got } => format!("want {want} got {got}"),
            Kind::Diverged { blob, total } => format!("largest diff blob {blob}px, {total}px total"),
            Kind::Sanity(n) => format!("{n}px differ"),
            _ => String::new(),
        }
    }
}

/// Panic messages carry indices/values that vary per case; group by location.
fn strip_msg(m: &str) -> String {
    match m.rsplit_once(" @ ") {
        Some((msg, loc)) => {
            let head: String = msg.chars().take_while(|c| !c.is_ascii_digit()).collect();
            format!("{} @ {loc}", head.trim_end())
        }
        None => m.to_string(),
    }
}

fn xor_blobs(a: &BinaryImage, b: &BinaryImage, unsure: &[&BinaryImage]) -> (usize, usize) {
    let mut d = BinaryImage::new_w_h(a.width, a.height);
    let mut total = 0;
    for y in 0..a.height {
        for x in 0..a.width {
            if a.get_pixel(x, y) != b.get_pixel(x, y) && !unsure.iter().any(|u| u.get_pixel(x, y)) {
                d.set_pixel(x, y, true);
                total += 1;
            }
        }
    }
    if total == 0 {
        return (0, 0);
    }
    let blob = d.to_clusters(true).iter().map(|c| c.size()).max().unwrap_or(0);
    (blob, total)
}

/// Shared state each worker exposes to the hang watchdog.
#[derive(Default)]
struct Slot {
    seed: u64,
    stage: &'static str,
    since: Option<Instant>,
    polys: Vec<Polygon>,
    reported: bool,
}

fn set_stage(slot: &Mutex<Slot>, stage: &'static str, polys: Option<&[Polygon]>) {
    let mut s = slot.lock().unwrap();
    s.stage = stage;
    if let Some(p) = polys {
        s.polys = p.to_vec();
    }
}

fn run_case(seed: u64, args: &Args, slot: &Mutex<Slot>, verbose: bool) -> (Vec<Finding>, usize) {
    let mut findings = vec![];
    let base = |kind: Kind| Finding {
        seed,
        mode: None,
        algo: None,
        kind,
        simple: true,
        points: 0,
        holes: 0,
        polys: vec![],
        tris: vec![],
        reference: None,
        got: None,
        mask: None,
        where_: String::new(),
    };

    set_stage(slot, "scene", None);
    let scene = gen_scene(seed, args.max_size, args.thin);

    set_stage(slot, "render scene", None);
    let image = match catch(|| render_scene(&scene.svg, scene.w, scene.h)) {
        Ok(Ok(img)) => img,
        Ok(Err(e)) => return (vec![base(Kind::HarnessPanic("render scene", e))], 0),
        Err(p) => return (vec![base(Kind::HarnessPanic("render scene", p))], 0),
    };

    set_stage(slot, "cluster", None);
    let comps = match catch(|| components(&image)) {
        Ok(c) => c,
        Err(p) => return (vec![base(Kind::HarnessPanic("cluster", p))], 0),
    };
    if verbose {
        println!("seed {seed}: {}x{}, {} components", scene.w, scene.h, comps.len());
    }

    let mut checked = 0;
    for comp in comps.iter() {
        let (pw, ph) = (comp.mask.width as u32, comp.mask.height as u32);
        for &mode in args.modes.iter() {
            let where_ = format!(
                "cluster {} piece {} at ({}, {}) size {}x{}",
                comp.cluster,
                comp.piece,
                comp.origin.0,
                comp.origin.1,
                pw - 2 * PAD as u32,
                ph - 2 * PAD as u32
            );
            set_stage(slot, "trace", None);
            let polys = match catch(|| trace(comp, mode)) {
                Ok(p) => p,
                Err(p) => {
                    let mut f = base(Kind::HarnessPanic("trace", p));
                    f.mode = Some(mode);
                    f.where_ = where_;
                    findings.push(f);
                    continue;
                }
            };
            if polys.is_empty() || polys.iter().any(|p| p.num_points() < 3) {
                // A traced ring with < 3 points is a tracer issue; nothing to triangulate.
                continue;
            }
            let rings: Vec<Vec<PointF64>> = polys.iter().map(ring_points).collect();
            let npoints: usize = rings.iter().map(|r| r.len()).sum();
            let nholes = polys.len() - 1;
            let simple = is_weakly_simple(&polys);
            let want_area: f64 = rings.iter().map(|r| signed_area(r).abs()).enumerate()
                .map(|(i, a)| if i == 0 { a } else { -a })
                .sum();

            let mk = |kind: Kind| Finding {
                seed,
                mode: Some(mode),
                algo: None,
                kind,
                simple,
                points: npoints,
                holes: nholes,
                polys: polys.clone(),
                tris: vec![],
                reference: None,
                got: None,
                mask: Some(comp.mask.clone()),
                where_: where_.clone(),
            };

            set_stage(slot, "render rings", None);
            let (ring_mask, ring_unsure) = match render_mask(&rings_svg(&rings, pw, ph), pw, ph) {
                Ok(m) => m,
                Err(e) => {
                    findings.push(mk(Kind::HarnessPanic("render rings", e)));
                    continue;
                }
            };
            let (reference, ref_unsure) = match mode {
                Mode::None => {
                    let (_, total) = xor_blobs(&ring_mask, &comp.mask, &[]);
                    if total > 0 {
                        let mut f = mk(Kind::Sanity(total));
                        f.reference = Some(comp.mask.clone());
                        f.got = Some(ring_mask.clone());
                        findings.push(f);
                    }
                    // Cairo samples pixel centres (x+0.5, y+0.5). Pixel-aligned rings put
                    // every boundary edge between centres, so nothing is a tie: the
                    // mask is exact and the ring render must match it bit for bit.
                    (comp.mask.clone(), BinaryImage::new_w_h(comp.mask.width, comp.mask.height))
                }
                Mode::Polygon => (ring_mask, ring_unsure),
            };

            for algo in [Algo::Ec, Algo::Mono, Algo::Opt] {
                if algo == Algo::Opt && nholes > 0 {
                    continue;
                }
                let input: &[Polygon] = if algo == Algo::Opt { &polys[..1] } else { &polys };
                set_stage(slot, algo.name(), Some(input));
                let result = catch(|| triangulate(algo, input));
                set_stage(slot, "compare", None);
                checked += 1;

                let mut f = mk(Kind::Error(String::new()));
                f.algo = Some(algo);
                let tris = match result {
                    Err(p) => {
                        f.kind = Kind::Panic(p);
                        findings.push(f);
                        continue;
                    }
                    Ok(Err(e)) => {
                        f.kind = Kind::Error(e);
                        findings.push(f);
                        continue;
                    }
                    Ok(Ok(t)) => t,
                };
                f.tris = tris.clone();

                if let Some(bad) = tris.iter().find(|t| {
                    t.num_points() != 3
                        || t.props().points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite())
                }) {
                    f.kind = Kind::Malformed(format!("triangle with {} points", bad.num_points()));
                    findings.push(f);
                    continue;
                }

                let got_area: f64 = tris.iter().map(|t| signed_area(&t.props().points).abs()).sum();
                if (got_area - want_area).abs() > 1e-6 * want_area.max(1.0) {
                    let mut a = f.clone();
                    a.kind = Kind::Area { want: want_area, got: got_area };
                    findings.push(a);
                }

                let tri_rings: Vec<Vec<PointF64>> = tris.iter().map(ring_points).collect();
                set_stage(slot, "render triangles", None);
                let (got, got_unsure) = match render_mask(&rings_svg(&tri_rings, pw, ph), pw, ph) {
                    Ok(m) => m,
                    Err(e) => {
                        f.kind = Kind::HarnessPanic("render triangles", e);
                        findings.push(f);
                        continue;
                    }
                };
                // In None mode an exact triangulation has the same pixel-aligned
                // boundary, so its render has no ties either; only interior
                // diagonals cross centres, and both sides of those are covered.
                let unsure: &[&BinaryImage] = match mode {
                    Mode::None => &[],
                    Mode::Polygon => &[&ref_unsure, &got_unsure],
                };
                let (blob, total) = xor_blobs(&reference, &got, unsure);
                if blob > args.tol {
                    f.kind = Kind::Diverged { blob, total };
                    f.reference = Some(reference.clone());
                    f.got = Some(got);
                    findings.push(f);
                }
            }
        }
    }
    (findings, checked)
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

fn dump_polys(polys: &[Polygon]) -> String {
    // Same text format as tests/polypartition.rs `parse_input`.
    let mut s = polys.len().to_string();
    for p in polys {
        s.push('\n');
        s.push_str(&p.props().dump(true));
    }
    s
}

fn write_png_diff(path: &Path, reference: &BinaryImage, got: &BinaryImage) -> Result<(), String> {
    let (w, h) = (reference.width, reference.height);
    let scale = (256 / w.max(h)).clamp(1, 16);
    let surf = cairo::ImageSurface::create(cairo::Format::Rgb24, (w * scale) as i32, (h * scale) as i32)
        .map_err(|e| e.to_string())?;
    {
        let cr = cairo::Context::new(&surf).map_err(|e| e.to_string())?;
        for y in 0..h {
            for x in 0..w {
                let (r, g) = (reference.get_pixel(x, y), got.get_pixel(x, y));
                let c = match (r, g) {
                    (true, true) => (0.25, 0.25, 0.25),
                    (true, false) => (0.9, 0.1, 0.1), // missing from triangulation
                    (false, true) => (0.1, 0.3, 0.95), // extra in triangulation
                    (false, false) => (1.0, 1.0, 1.0),
                };
                cr.set_source_rgb(c.0, c.1, c.2);
                cr.rectangle((x * scale) as f64, (y * scale) as f64, scale as f64, scale as f64);
                cr.fill().map_err(|e| e.to_string())?;
            }
        }
    }
    surf.flush();
    let mut file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    surf.write_to_png(&mut file).map_err(|e| e.to_string())
}

fn save(dir: &Path, f: &Finding, args: &Args) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let scene = gen_scene(f.seed, args.max_size, args.thin);
    std::fs::write(dir.join("scene.svg"), &scene.svg)?;
    let mut report = format!(
        "signature: {}\nseed: {}\nwhere: {}\npoints: {}  holes: {}  weakly simple: {}\n{}\nreplay: cargo run --release -- --case {} --max-size {}{}\n",
        f.signature(),
        f.seed,
        f.where_,
        f.points,
        f.holes,
        f.simple,
        f.short(),
        f.seed,
        args.max_size,
        if args.thin { " --thin" } else { "" }
    );
    if let Kind::Panic(m) | Kind::Error(m) | Kind::HarnessPanic(_, m) = &f.kind {
        let _ = writeln!(report, "message: {m}");
    }
    std::fs::write(dir.join("report.txt"), report)?;
    if let Some(m) = &f.mask {
        // The cluster itself, unpadded: '#' set, '.' clear.
        let mut txt = String::new();
        for y in PAD as usize..m.height - PAD as usize {
            for x in PAD as usize..m.width - PAD as usize {
                txt.push(if m.get_pixel(x, y) { '#' } else { '.' });
            }
            txt.push('\n');
        }
        std::fs::write(dir.join("mask.txt"), txt)?;
    }
    if !f.polys.is_empty() {
        std::fs::write(dir.join("input.txt"), dump_polys(&f.polys))?;
    }
    if !f.tris.is_empty() {
        let rings: Vec<Vec<PointF64>> = f.tris.iter().map(ring_points).collect();
        let (w, h) = f.reference.as_ref().map(|r| (r.width as u32, r.height as u32)).unwrap_or((64, 64));
        std::fs::write(dir.join("triangles.svg"), rings_svg(&rings, w, h))?;
    }
    if let (Some(r), Some(g)) = (&f.reference, &f.got) {
        let _ = write_png_diff(&dir.join("diff.png"), r, g);
    }
    Ok(())
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').chars().take(80).collect()
}

#[derive(Default)]
struct Summary {
    /// signature -> (count, smallest repro size, seed of smallest)
    by_sig: BTreeMap<String, (u64, usize, u64)>,
    cases: u64,
    checked: u64,
}

fn record(summary: &Mutex<Summary>, findings: Vec<Finding>, checked: usize, args: &Args, verbose: bool) {
    let mut s = summary.lock().unwrap();
    s.cases += 1;
    s.checked += checked as u64;
    // Dedupe within a case so one bad image doesn't dominate the counts.
    let mut seen: HashMap<String, ()> = HashMap::new();
    for f in findings {
        let sig = f.signature();
        let size = if f.points == 0 { usize::MAX } else { f.points };
        let fresh = seen.insert(sig.clone(), ()).is_none();
        let entry = s.by_sig.entry(sig.clone()).or_insert((0, usize::MAX, f.seed));
        if fresh {
            entry.0 += 1;
        }
        let smaller = size < entry.1 || entry.0 == 1 && fresh && entry.1 == usize::MAX;
        if smaller || verbose {
            entry.1 = entry.1.min(size);
            entry.2 = f.seed;
            let dir = if verbose {
                args.out.join(format!("case-{}", f.seed)).join(format!("{}-{}", slug(&sig), seen.len()))
            } else {
                args.out.join(slug(&sig))
            };
            if !verbose {
                let _ = std::fs::remove_dir_all(&dir);
            }
            if let Err(e) = save(&dir, &f, args) {
                eprintln!("failed to save {}: {e}", dir.display());
            }
            if verbose || entry.0 == 1 {
                println!("  {sig}  seed={} {} {}", f.seed, f.where_, f.short());
            }
        }
    }
}

fn print_summary(s: &Summary, elapsed: Duration) {
    println!(
        "\n{} cases, {} triangulations checked in {:.1}s",
        s.cases,
        s.checked,
        elapsed.as_secs_f64()
    );
    if s.by_sig.is_empty() {
        println!("no findings");
        return;
    }
    println!("{:>7}  {:>8}  {:>10}  signature", "cases", "min pts", "min seed");
    for (sig, (count, size, seed)) in s.by_sig.iter() {
        let size = if *size == usize::MAX { "-".into() } else { size.to_string() };
        println!("{count:>7}  {size:>8}  {seed:>10}  {sig}");
    }
}

fn main() {
    let args = parse_args();
    install_panic_hook();

    if let Some(path) = &args.render {
        let svg = std::fs::read_to_string(path).expect("read svg");
        let dim = |k: &str| -> u32 {
            let i = svg.find(&format!(" {k}=\"")).expect("svg needs width/height") + k.len() + 3;
            svg[i..].split('"').next().unwrap().parse().expect("integer width/height")
        };
        let (w, h) = (dim("width"), dim("height"));
        let (data, stride) = rsvg_render(&svg, w, h).expect("render");
        let m = {
            let mut m = BinaryImage::new_w_h(w as usize, h as usize);
            for y in 0..h as usize {
                for x in 0..w as usize {
                    let px = u32::from_ne_bytes(data[y * stride + x * 4..][..4].try_into().unwrap());
                    m.set_pixel(x, y, (px >> 24) >= 128 && ((px >> 16) & 0xff) < 128);
                }
            }
            m
        };
        for y in 0..h as usize {
            let row: String = (0..w as usize).map(|x| if m.get_pixel(x, y) { '#' } else { '.' }).collect();
            println!("{row}");
        }
        return;
    }

    if let Some(path) = &args.input {
        replay_input(&std::fs::read_to_string(path).expect("read input"));
        return;
    }

    if let Some(seed) = args.case {
        let slot = Mutex::new(Slot::default());
        let summary = Mutex::new(Summary::default());
        let start = Instant::now();
        let (findings, checked) = run_case(seed, &args, &slot, true);
        record(&summary, findings, checked, &args, true);
        print_summary(&summary.lock().unwrap(), start.elapsed());
        return;
    }

    std::fs::create_dir_all(&args.out).expect("create --out dir");
    println!(
        "fuzzing from seed {} ({} iters, {} threads, modes {:?}, tol {}) -> {}",
        args.seed,
        if args.iters == 0 { "∞".to_string() } else { args.iters.to_string() },
        args.threads,
        args.modes,
        args.tol,
        args.out.display()
    );

    let next = Arc::new(AtomicU64::new(0));
    let summary = Arc::new(Mutex::new(Summary::default()));
    let slots: Vec<Arc<Mutex<Slot>>> = (0..args.threads).map(|_| Arc::new(Mutex::new(Slot::default()))).collect();
    let start = Instant::now();

    let handles: Vec<_> = slots
        .iter()
        .map(|slot| {
            let (next, summary, slot, args) = (next.clone(), summary.clone(), slot.clone(), args.clone());
            std::thread::spawn(move || loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if args.iters != 0 && i >= args.iters {
                    slot.lock().unwrap().since = None;
                    return;
                }
                let seed = args.seed + i;
                {
                    let mut s = slot.lock().unwrap();
                    s.seed = seed;
                    s.since = Some(Instant::now());
                    s.reported = false;
                }
                let (findings, checked) = run_case(seed, &args, &slot, false);
                record(&summary, findings, checked, &args, false);
            })
        })
        .collect();

    // Watchdog + progress.
    let mut hung = 0;
    let mut last_progress = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(200));
        let running = handles.iter().filter(|h| !h.is_finished()).count();
        for slot in slots.iter() {
            let mut s = slot.lock().unwrap();
            if let Some(since) = s.since {
                if !s.reported && since.elapsed() > args.timeout {
                    s.reported = true;
                    hung += 1;
                    let f = Finding {
                        seed: s.seed,
                        mode: None,
                        algo: None,
                        kind: Kind::Hang(s.stage),
                        simple: true,
                        points: s.polys.iter().map(|p| p.num_points()).sum(),
                        holes: s.polys.len().saturating_sub(1),
                        polys: s.polys.clone(),
                        tris: vec![],
                        reference: None,
                        got: None,
                        mask: None,
                        where_: String::new(),
                    };
                    println!("  HANG seed={} in {} ({}s)", s.seed, s.stage, since.elapsed().as_secs());
                    drop(s);
                    record(&summary, vec![f], 0, &args, false);
                }
            }
        }
        if last_progress.elapsed() > Duration::from_secs(10) {
            last_progress = Instant::now();
            let s = summary.lock().unwrap();
            println!(
                "[{:.0}s] {} cases, {} triangulations, {} signatures",
                start.elapsed().as_secs_f64(),
                s.cases,
                s.checked,
                s.by_sig.len()
            );
        }
        if running <= hung {
            break;
        }
    }

    let s = summary.lock().unwrap();
    print_summary(&s, start.elapsed());
    let failed = !s.by_sig.is_empty();
    drop(s);
    if hung > 0 {
        // Hung workers can't be joined; bail out.
        std::process::exit(3);
    }
    for h in handles {
        let _ = h.join();
    }
    std::process::exit(if failed { 1 } else { 0 });
}
