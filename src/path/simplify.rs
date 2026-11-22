use std::cmp::Ordering;

use crate::{PathI32, PointI32};

use super::util::signed_area;

pub struct PathSimplify;

#[derive(Copy, Clone, Debug)]
pub enum PathSimplifyMode {
    None,
    Polygon,
    Spline,
}

#[derive(Copy, Clone)]
pub enum Dir {
    Up,
    Right,
    Down,
    Left
}

impl Default for PathSimplifyMode {
    fn default() -> Self {
        Self::None
    }
}

impl PathSimplify {

    /// Returns a copy of a path after removing 1-pixel staircases.
    ///
    /// Clockwiseness of path must be indicated to perform outset.
    ///
    /// If that leaves edges crossing or overlapping (a spike, or a 1px feature
    /// folded flat), the corners whose removal caused it are kept instead.
    pub fn remove_staircase(path: &PathI32, clockwise: bool) -> PathI32 {
        let unchecked = Self::remove_staircase_in(path, clockwise, 0, None);
        if !crossing_rings(std::slice::from_ref(&unchecked), &[false])[0] {
            return unchecked;
        }
        let mut index = SegmentIndex::new(std::slice::from_ref(path));
        index.insert_ring(path, 0);
        Self::remove_staircase_in(path, clockwise, 0, Some(&mut index))
    }

    fn remove_staircase_in(path: &PathI32, clockwise: bool, ring: usize, mut index: Option<&mut SegmentIndex>) -> PathI32 {
        let path = &path.path;
        let len = path.len();

        let segment_length = |i: usize, j: usize| -> i32 {
            (path[i].x - path[j].x).abs() + (path[i].y - path[j].y).abs()
        };

        let mut result = PathI32::new();
        if len == 0 {
            return result;
        }
        let mut last = 0;
        let mut chord = None;
        for i in 0..len {
            let j = (i + 1) % len;
            let h = if i > 0 { i - 1 } else { len - 1 };
            let mut keep = if i == 0 || i == len - 1 {
                true
            } else if segment_length(i, h) == 1 || segment_length(i, j) == 1 {
                let area = signed_area(path[h], path[i], path[j]);
                area != 0 && (area > 0) == clockwise
            } else {
                true
            };
            if !keep {
                if let Some(index) = index.as_mut() {
                    // Dropping i joins the last kept corner to i + 1.
                    if index.blocks(path[last], path[j], ring, last, j) {
                        keep = true;
                    } else {
                        if let Some(id) = chord {
                            index.kill(id);
                        }
                        chord = index.replace(path[last], path[j], ring, last, j);
                    }
                }
            }
            if keep {
                result.add(path[i]);
                last = i;
                chord = None;
            }
        }
        result
    }

    /// Simplifies a cluster's outline and holes together, as
    /// [`PathI32::simplify`] does for one path: staircases removed, then
    /// penalties limited.
    ///
    /// Simplified independently, two sides of a 1-2px feature are within
    /// tolerance of each other, and a chord along one can cut across the
    /// other, or across a neighbouring ring. Where that happens, the rings
    /// involved are simplified again with every new edge required to stay
    /// clear of all others, so rings that were simple and disjoint stay so.
    /// Everywhere else the result is exactly the plain one. Rings of a few
    /// pixels that have always folded down to a point or a sliver still do,
    /// and are left out of the checks.
    pub fn simplify_jointly(paths: &[PathI32], clockwise: &[bool]) -> Vec<PathI32> {
        let unchecked: Vec<PathI32> = paths
            .iter()
            .zip(clockwise)
            .map(|(path, &cw)| Self::limit_penalties_in(&Self::remove_staircase_in(path, cw, 0, None), 0, None))
            .collect();
        let collapsed: Vec<bool> = unchecked.iter().map(|path| distinct_points(path) < 3).collect();
        // Almost always the plain result is fine, and then it is the result.
        // Otherwise only the rings involved are redone, checked against each
        // other and against the plain result of the rest.
        let redo = crossing_rings(&unchecked, &collapsed);
        if !redo.iter().any(|&r| r) {
            return unchecked;
        }
        // A kept ring's chord can still cut the raw outline of a redone one:
        // then redo that ring too, and in the end every ring.
        let mut redo = redo;
        for _ in 0..4 {
            let result = Self::redo_checked(paths, clockwise, &unchecked, &collapsed, &redo);
            let still = crossing_rings(&result, &collapsed);
            if !still.iter().any(|&r| r) {
                return result;
            }
            for (r, s) in redo.iter_mut().zip(still) {
                *r |= s;
            }
        }
        let all: Vec<bool> = collapsed.iter().map(|&c| !c).collect();
        Self::redo_checked(paths, clockwise, &unchecked, &collapsed, &all)
    }

    /// Simplifies the rings marked `redo` again with every new edge checked
    /// against all live edges; the others keep their `unchecked` result,
    /// which collapsed rings take no part in.
    fn redo_checked(
        paths: &[PathI32], clockwise: &[bool], unchecked: &[PathI32], collapsed: &[bool], redo: &[bool],
    ) -> Vec<PathI32> {
        let fixed = |r: usize| !redo[r] && !collapsed[r];
        let mut index = SegmentIndex::new(paths);
        for (r, path) in paths.iter().enumerate() {
            if redo[r] {
                index.insert_ring(path, r);
            } else if fixed(r) {
                index.insert_ring(&unchecked[r], r);
            }
        }
        let staircased: Vec<Option<PathI32>> = (0..paths.len())
            .map(|r| redo[r].then(|| Self::remove_staircase_in(&paths[r], clockwise[r], r, Some(&mut index))))
            .collect();

        let current = |r: usize| staircased[r].as_ref().unwrap_or(&unchecked[r]);
        let mut index = SegmentIndex::new(&(0..paths.len()).map(|r| current(r).clone()).collect::<Vec<_>>());
        for r in (0..paths.len()).filter(|&r| redo[r] || fixed(r)) {
            index.insert_ring(current(r), r);
        }
        (0..paths.len())
            .map(|r| match &staircased[r] {
                Some(path) => Self::limit_penalties_in(path, r, Some(&mut index)),
                None => unchecked[r].clone(),
            })
            .collect()
    }

    /// Returns a copy of a path with runs of vertices replaced by chords
    /// wherever the chord stays within tolerance of the run.
    ///
    /// If the chords would leave edges crossing or overlapping (on a 1-2px
    /// wide feature the two sides are within tolerance of each other), the
    /// path is redone taking only chords that stay clear of every other edge.
    pub fn limit_penalties(path: &PathI32) -> PathI32 {
        Self::limit_penalties_jointly(std::slice::from_ref(path)).pop().unwrap()
    }

    /// [`PathSimplify::limit_penalties`] for several rings at once, such as a
    /// cluster's outline and its holes, so that rings which did not cross
    /// before do not cross after.
    pub fn limit_penalties_jointly(paths: &[PathI32]) -> Vec<PathI32> {
        // Rings of a few pixels fold down to a point or a sliver; that has
        // always been their simplified form, and splines rely on it to drop
        // specks. Keep it, and leave such rings out of the crossing checks.
        let unchecked: Vec<PathI32> = paths.iter().map(|path| Self::limit_penalties_in(path, 0, None)).collect();
        let collapsed: Vec<bool> = unchecked.iter().map(|path| distinct_points(path) < 3).collect();
        if !crossing_rings(&unchecked, &collapsed).iter().any(|&r| r) {
            return unchecked;
        }
        let mut index = SegmentIndex::new(paths);
        for (r, path) in paths.iter().enumerate() {
            if !collapsed[r] {
                index.insert_ring(path, r);
            }
        }
        paths
            .iter()
            .zip(unchecked)
            .enumerate()
            .map(|(r, (path, unchecked))| {
                if collapsed[r] {
                    unchecked
                } else {
                    Self::limit_penalties_in(path, r, Some(&mut index))
                }
            })
            .collect()
    }

    fn limit_penalties_in(path: &PathI32, ring: usize, index: Option<&mut SegmentIndex>) -> PathI32 {
        let tolerance = 1.0;
        let path = &path.path;
        let len = path.len();
        let past_delta = |from: usize, to: usize| -> f64 {
            (from..to).skip(1).map(|i| {
                Self::evaluate_penalty(path[from], path[i], path[to])
            }).fold(0.0, |a, b| a.max(b)) // find max
        };

        let mut result = PathI32::new();
        let index = match index {
            Some(index) if len >= 3 => index,
            _ => {
                // The plain greedy pass.
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
                        result.add(path[i-1]);
                    }
                    if i == len - 1 {
                        result.add(path[i]);
                    }
                }
                return result;
            }
        };

        // The same greedy pass, but each chord is checked once it is about to
        // be emitted, stepping back while it would touch another edge. Where
        // nothing is in the way, the output is exactly the unchecked one.
        result.add(path[0]);
        let mut last = 0;
        while last < len - 1 {
            let mut end = last + 1;
            // A NaN penalty (a chord back onto its own start) passes, as in
            // the plain pass.
            while end + 1 < len
                && !matches!(past_delta(last, end + 1).partial_cmp(&tolerance), Some(Ordering::Greater | Ordering::Equal))
            {
                end += 1;
            }
            while end > last + 1 && index.blocks(path[last], path[end], ring, last, end) {
                end -= 1;
            }
            index.replace(path[last], path[end], ring, last, end);
            result.add(path[end]);
            last = end;
        }
        result
    }

    fn evaluate_penalty(a: PointI32, b: PointI32, c: PointI32) -> f64 {
        let sq = |x| { (x * x) as f64 };
        let l1 = (sq(a.x - b.x) + sq(a.y - b.y)).sqrt();
        let l2 = (sq(b.x - c.x) + sq(b.y - c.y)).sqrt();
        let l3 = (sq(c.x - a.x) + sq(c.y - a.y)).sqrt();
        let p = (l1 + l2 + l3) / 2.0;
        let area = (p * (p - l1) * (p - l2) * (p - l3)).sqrt();
        area * area / l3
    }
}
struct Segment {
    a: PointI32,
    b: PointI32,
    /// (ring, index) of an original edge; `None` for a chord.
    original: Option<(usize, usize)>,
    alive: bool,
}

/// Segments bucketed on a coarse grid, to ask whether a candidate chord would
/// touch any edge currently in a set of rings.
struct SegmentIndex {
    segments: Vec<Segment>,
    /// Per ring, the segment of each original edge.
    originals: Vec<Vec<usize>>,
    /// Segments overlapping each cell, row-major from `origin`.
    grid: Vec<Vec<usize>>,
    origin: PointI32,
    columns: i32,
    /// Per segment, the last query that looked at it.
    stamps: std::cell::RefCell<(u32, Vec<u32>)>,
}

impl SegmentIndex {
    const CELL: i32 = 8;

    /// An empty index covering every point of `paths`; chords only ever join
    /// such points.
    fn new(paths: &[PathI32]) -> Self {
        let mut points = paths.iter().flat_map(|p| p.path.iter());
        let first = points.next().copied().unwrap_or_default();
        let (mut lo, mut hi) = (first, first);
        for p in points {
            lo = PointI32::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = PointI32::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        let columns = (hi.x - lo.x) / Self::CELL + 1;
        let rows = (hi.y - lo.y) / Self::CELL + 1;
        Self {
            segments: vec![],
            originals: vec![vec![]; paths.len()],
            grid: vec![vec![]; (columns * rows) as usize],
            origin: lo,
            columns,
            stamps: Default::default(),
        }
    }

    fn cells(&self, a: PointI32, b: PointI32) -> impl Iterator<Item = usize> {
        let (o, columns) = (self.origin, self.columns);
        let (x0, x1) = ((a.x.min(b.x) - o.x) / Self::CELL, (a.x.max(b.x) - o.x) / Self::CELL);
        let (y0, y1) = ((a.y.min(b.y) - o.y) / Self::CELL, (a.y.max(b.y) - o.y) / Self::CELL);
        (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| (y * columns + x) as usize))
    }

    fn insert_ring(&mut self, path: &PathI32, ring: usize) {
        let ids = (0..path.path.len().saturating_sub(1))
            .map(|k| self.insert(path.path[k], path.path[k + 1], Some((ring, k))))
            .collect();
        self.originals[ring] = ids;
    }

    fn kill(&mut self, id: usize) {
        self.segments[id].alive = false;
    }

    fn insert(&mut self, a: PointI32, b: PointI32, original: Option<(usize, usize)>) -> usize {
        let id = self.segments.len();
        self.segments.push(Segment { a, b, original, alive: true });
        let (o, columns) = (self.origin, self.columns);
        let (x0, x1) = ((a.x.min(b.x) - o.x) / Self::CELL, (a.x.max(b.x) - o.x) / Self::CELL);
        let (y0, y1) = ((a.y.min(b.y) - o.y) / Self::CELL, (a.y.max(b.y) - o.y) / Self::CELL);
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.grid[(y * columns + x) as usize].push(id);
            }
        }
        id
    }

    /// Replaces the original edges `from..to` of `ring` by the chord between
    /// their ends, returning the chord (none for a single edge).
    fn replace(&mut self, a: PointI32, b: PointI32, ring: usize, from: usize, to: usize) -> Option<usize> {
        if to <= from + 1 {
            return None;
        }
        for k in from..to {
            let id = self.originals[ring][k];
            self.segments[id].alive = false;
        }
        Some(self.insert(a, b, None))
    }

    /// Whether the chord `a`-`b` replacing original edges `from..to` of
    /// `ring` would touch any other live edge anywhere but at its own ends.
    fn blocks(&self, a: PointI32, b: PointI32, ring: usize, from: usize, to: usize) -> bool {
        let mut stamps = self.stamps.borrow_mut();
        let (query, seen) = &mut *stamps;
        *query += 1;
        seen.resize(self.segments.len(), 0);
        for cell in self.cells(a, b) {
            for &id in self.grid[cell].iter() {
                if seen[id] == *query {
                    continue;
                }
                seen[id] = *query;
                let segment = &self.segments[id];
                if !segment.alive || matches!(segment.original, Some((r, k)) if r == ring && k >= from && k < to) {
                    continue;
                }
                if touches_open(a, b, segment.a, segment.b) {
                    return true;
                }
            }
        }
        false
    }
}

/// The rings (of those not `skip`ped) with an edge that crosses another edge
/// properly or overlaps it along a stretch, including a ring doubling back
/// on itself.
fn crossing_rings(paths: &[PathI32], skip: &[bool]) -> Vec<bool> {
    let mut involved = vec![false; paths.len()];
    let mut segments: Vec<(PointI32, PointI32)> = vec![];
    let mut ring_of: Vec<usize> = vec![];
    for (r, path) in paths.iter().enumerate() {
        if !skip[r] {
            for w in path.path.windows(2) {
                segments.push((w[0], w[1]));
                ring_of.push(r);
            }
        }
    }
    if segments.len() < 2 {
        return involved;
    }
    // A flat grid bucketed by counting sort: built once, never grown, with
    // about one cell per segment however sparse the bounding box is.
    let (mut lo, mut hi) = (segments[0].0, segments[0].0);
    for &(a, b) in segments.iter() {
        for p in [a, b] {
            lo = PointI32::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = PointI32::new(hi.x.max(p.x), hi.y.max(p.y));
        }
    }
    let cell = 8;
    let columns = ((hi.x - lo.x) / cell + 1) as usize;
    let rows = ((hi.y - lo.y) / cell + 1) as usize;
    let span = |(a, b): (PointI32, PointI32)| {
        let (x0, x1) = (((a.x.min(b.x) - lo.x) / cell) as usize, ((a.x.max(b.x) - lo.x) / cell) as usize);
        let (y0, y1) = (((a.y.min(b.y) - lo.y) / cell) as usize, ((a.y.max(b.y) - lo.y) / cell) as usize);
        (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| y * columns + x))
    };
    let mut starts = vec![0usize; columns * rows + 1];
    for &seg in segments.iter() {
        for cell in span(seg) {
            starts[cell + 1] += 1;
        }
    }
    for c in 0..columns * rows {
        starts[c + 1] += starts[c];
    }
    let mut fill = starts.clone();
    let mut ids = vec![0usize; starts[columns * rows]];
    for (id, &seg) in segments.iter().enumerate() {
        for cell in span(seg) {
            ids[fill[cell]] = id;
            fill[cell] += 1;
        }
    }
    let mut seen = vec![0usize; segments.len()];
    for (id, &(a, b)) in segments.iter().enumerate() {
        for cell in span((a, b)) {
            for &other in &ids[starts[cell]..starts[cell + 1]] {
                if other <= id || seen[other] == id + 1 {
                    continue;
                }
                seen[other] = id + 1;
                let (c, d) = segments[other];
                if c.x.max(d.x) < a.x.min(b.x) || c.x.min(d.x) > a.x.max(b.x)
                    || c.y.max(d.y) < a.y.min(b.y) || c.y.min(d.y) > a.y.max(b.y)
                {
                    continue;
                }
                if crosses_or_overlaps(a, b, c, d) {
                    involved[ring_of[id]] = true;
                    involved[ring_of[other]] = true;
                }
            }
        }
    }
    involved
}

/// A proper crossing, or a collinear overlap of positive length.
fn crosses_or_overlaps(a: PointI32, b: PointI32, c: PointI32, d: PointI32) -> bool {
    let (o1, o2) = (orient(a, b, c), orient(a, b, d));
    if o1 == 0 && o2 == 0 {
        let along = |p: PointI32| (p.x - a.x) as i64 * (b.x - a.x) as i64 + (p.y - a.y) as i64 * (b.y - a.y) as i64;
        let ab = along(b);
        return along(c).min(along(d)).max(0) < along(c).max(along(d)).min(ab);
    }
    let (o3, o4) = (orient(c, d, a), orient(c, d, b));
    ((o1 > 0 && o2 < 0) || (o1 < 0 && o2 > 0)) && ((o3 > 0 && o4 < 0) || (o3 < 0 && o4 > 0))
}

/// Distinct vertices of a closed path, ignoring repeats and the closing point.
fn distinct_points(path: &PathI32) -> usize {
    let mut pts: Vec<PointI32> = vec![];
    for &p in path.path.iter() {
        if pts.last() != Some(&p) {
            pts.push(p);
        }
    }
    while pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    pts.len()
}

fn orient(a: PointI32, b: PointI32, c: PointI32) -> i64 {
    (b.x - a.x) as i64 * (c.y - a.y) as i64 - (b.y - a.y) as i64 * (c.x - a.x) as i64
}

/// The closed segment `c`-`d` meets the open segment `a`-`b`: a proper
/// crossing, an end of `c`-`d` inside `a`-`b`, or a collinear overlap.
fn touches_open(a: PointI32, b: PointI32, c: PointI32, d: PointI32) -> bool {
    let (o1, o2) = (orient(a, b, c), orient(a, b, d));
    if (o1 > 0 && o2 > 0) || (o1 < 0 && o2 < 0) {
        return false;
    }
    let ab = (b.x - a.x) as i64 * (b.x - a.x) as i64 + (b.y - a.y) as i64 * (b.y - a.y) as i64;
    let along = |p: PointI32| (p.x - a.x) as i64 * (b.x - a.x) as i64 + (p.y - a.y) as i64 * (b.y - a.y) as i64;
    if o1 == 0 && o2 == 0 {
        let (tc, td) = (along(c), along(d));
        return tc.min(td).max(0) < tc.max(td).min(ab);
    }
    if (o1 == 0 && along(c) > 0 && along(c) < ab) || (o2 == 0 && along(d) > 0 && along(d) < ab) {
        return true;
    }
    let (o3, o4) = (orient(c, d, a), orient(c, d, b));
    ((o1 > 0 && o2 < 0) || (o1 < 0 && o2 > 0)) && ((o3 > 0 && o4 < 0) || (o3 < 0 && o4 > 0))
}
