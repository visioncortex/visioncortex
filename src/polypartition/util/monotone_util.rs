use std::{cell::RefCell, rc::Rc};

use crate::PointF64;

use crate::polypartition::VertexType;

use super::{f64_approximately, is_convex, is_strictly_between, orient, point_f64_approximately, repair_pinch};

#[derive(Clone, Default, PartialEq)]
pub struct MonotoneVertex {
    pub p: PointF64,
    /// Symbolic perturbation: the vertex is taken to sit at `p + ε·d` for an
    /// infinitesimal ε. Zero except where several boundary passes meet at
    /// one point, where it points into this pass's own sector so the copies
    /// come apart without crossing. See [`SweepPoint`].
    pub d: PointF64,
    pub previous: usize,
    pub next: usize,
}

impl MonotoneVertex {
    pub fn sweep_point(&self, index: usize) -> SweepPoint {
        SweepPoint { p: self.p, d: self.d, index }
    }
}

/// A point as the monotone sweep sees it.
///
/// The sweep runs in decreasing y, breaking ties in y by x, as if the plane
/// were sheared by an infinitesimal θ. Pixel outlines also
/// bring points shared by several boundary passes (a hole touching the
/// outline at a corner, a diagonal pinch). Those copies are pulled apart by a
/// second, much smaller ε ≪ θ along `d`, and any remaining tie goes to the
/// index. Every order and orientation test is evaluated under this
/// perturbation, so none of them is ever an unresolved tie.
#[derive(Clone, Copy, Debug, Default)]
pub struct SweepPoint {
    pub p: PointF64,
    pub d: PointF64,
    pub index: usize,
}

fn cross(u: PointF64, w: PointF64) -> f64 {
    u.x * w.y - u.y * w.x
}

fn sign(v: f64) -> i32 {
    if v > 0.0 {
        1
    } else if v < 0.0 {
        -1
    } else {
        0
    }
}

/// Sign of `orient(a, b, c)` under the perturbation: the first non-zero
/// coefficient of its expansion in ε.
pub fn orient_sign(a: &SweepPoint, b: &SweepPoint, c: &SweepPoint) -> i32 {
    let (u, w) = (b.p - a.p, c.p - a.p);
    let s = sign(cross(u, w));
    if s != 0 {
        return s;
    }
    let (du, dw) = (b.d - a.d, c.d - a.d);
    let s = sign(cross(u, dw) + cross(du, w));
    if s != 0 {
        return s;
    }
    sign(cross(du, dw))
}

/// `a` comes strictly before `b` in the sweep. As in upstream (written for
/// y-up coordinates), the sweep starts at the largest y, ties going to the
/// larger x.
pub fn sweep_before(a: &SweepPoint, b: &SweepPoint) -> bool {
    if a.p.y != b.p.y {
        return a.p.y > b.p.y;
    }
    if a.p.x != b.p.x {
        return a.p.x > b.p.x;
    }
    if a.d.y != b.d.y {
        return a.d.y > b.d.y;
    }
    if a.d.x != b.d.x {
        return a.d.x > b.d.x;
    }
    a.index > b.index
}

#[derive(Clone, Debug, Default)]
pub struct ScanLineEdge {
    pub index: usize,
    pub p1: PointF64,
    pub p2: PointF64,
    /// Perturbations of `p1` and `p2`, see [`MonotoneVertex::d`].
    pub d1: PointF64,
    pub d2: PointF64,
}

impl ScanLineEdge {
    pub fn is_left_of(&self, other: &ScanLineEdge) -> bool {
        if self.is_same_position_as(other) {
            return self.index < other.index;
        }

        if f64_approximately(other.p1.y, other.p2.y) {
            if f64_approximately(self.p1.y, self.p2.y) {
                return self.p1.y < other.p1.y;
            }
            return is_convex(&self.p1, &self.p2, &other.p1);
        }

        if f64_approximately(self.p1.y, self.p2.y) || self.p1.y < other.p1.y {
            return !is_convex(&other.p1, &other.p2, &self.p1);
        }

        is_convex(&self.p1, &self.p2, &other.p1)
    }

    pub fn is_same_position_as(&self, other: &ScanLineEdge) -> bool {
        point_f64_approximately(self.p1, other.p1) && point_f64_approximately(self.p2, other.p2)
    }

    fn ends(&self) -> (SweepPoint, SweepPoint) {
        (
            SweepPoint { p: self.p1, d: self.d1, index: 0 },
            SweepPoint { p: self.p2, d: self.d2, index: 0 },
        )
    }

    /// Side of `p` relative to this edge (`p1` is its earlier end in the
    /// sweep): negative when `p` is to its right, i.e. at larger x.
    fn side_of(&self, p: &SweepPoint) -> i32 {
        let (a, b) = self.ends();
        -orient_sign(&a, &b, p)
    }

    /// For two edges both crossing the sweep line and not crossing each
    /// other: whether `self` is to the right of `other` along it.
    fn is_right_of(&self, other: &ScanLineEdge) -> bool {
        let (a, b) = self.ends();
        let (s1, s2) = (other.side_of(&a), other.side_of(&b));
        if s1 <= 0 && s2 <= 0 && (s1, s2) != (0, 0) {
            return true;
        }
        if s1 >= 0 && s2 >= 0 && (s1, s2) != (0, 0) {
            return false;
        }
        // `self` straddles the line through `other`, so `other` lies to one
        // side of the line through `self`.
        let (c, d) = other.ends();
        let (t1, t2) = (self.side_of(&c), self.side_of(&d));
        t1 >= 0 && t2 >= 0
    }
}

impl PartialEq for ScanLineEdge {
    fn eq(&self, other: &Self) -> bool {
        self.is_same_position_as(other)
    }
}

impl Eq for ScanLineEdge {}

impl PartialOrd for ScanLineEdge {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScanLineEdge {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.is_same_position_as(other) {
            std::cmp::Ordering::Equal
        } else if self.is_left_of(other) {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    }
}

// Returns true iff p1 is considered to be above p2
pub fn is_above(p1: &PointF64, p2: &PointF64) -> bool {
    p1.y < p2.y || (p1.y == p2.y && p1.x < p2.x)
}

#[allow(clippy::too_many_arguments)]
pub fn add_diagonal(vertices: &mut Vec<MonotoneVertex>, num_vertices: &mut usize,
    index1: usize, index2: usize, vertex_types: &mut Vec<VertexType>,
    edge_vec_pointers: &mut Vec<Option<EdgeVecPtr>>, helpers: &mut Vec<usize>) {

    let new_index1 = *num_vertices;
    *num_vertices += 1;
    let new_index2 = *num_vertices;
    *num_vertices += 1;

    vertices[new_index1].p = vertices[index1].p;
    vertices[new_index2].p = vertices[index2].p;
    // A copy is the same (perturbed) point.
    vertices[new_index1].d = vertices[index1].d;
    vertices[new_index2].d = vertices[index2].d;

    vertices[new_index2].next = vertices[index2].next;
    vertices[new_index1].next = vertices[index1].next;

    let temp_index = vertices[index2].next;
    vertices[temp_index].previous = new_index2;
    let temp_index = vertices[index1].next;
    vertices[temp_index].previous = new_index1;

    vertices[index1].next = new_index2;
    vertices[new_index2].previous = index1;

    vertices[index2].next = new_index1;
    vertices[new_index1].previous = index2;

    // Update all relevant structures
    vertex_types[new_index1] = vertex_types[index1];
    edge_vec_pointers[new_index1] = edge_vec_pointers[index1].clone();
    helpers[new_index1] = helpers[index1];
    if let Some(edge) = &edge_vec_pointers[new_index1] {
        edge.borrow_mut().index = new_index1;
    }

    vertex_types[new_index2] = vertex_types[index2];
    edge_vec_pointers[new_index2] = edge_vec_pointers[index2].clone();
    helpers[new_index2] = helpers[index2];
    if let Some(edge) = &edge_vec_pointers[new_index2] {
        edge.borrow_mut().index = new_index2;
    }
}

/// A data structure to store ScanLineEdge's
/// Requirement:
/// 1) Change any element given Rc in constant time (supported by Rc<RefCell<...>> in nature)
/// 2) Insert
/// 3) Remove given Rc
/// 4) Lowerbound search
#[derive(Default)]
pub struct EdgeVec {
    pub vec: Vec<EdgeVecPtr>,
}

pub type EdgeVecPtr = Rc<RefCell<ScanLineEdge>>;

impl EdgeVec {
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_edge_copy(&self, index: usize) -> Option<ScanLineEdge> {
        if index < self.vec.len() {
            Some(self.vec[index].borrow().clone())
        } else {
            None
        }
    }

    pub fn insert(&mut self, edge: ScanLineEdge) -> Option<EdgeVecPtr> {
        if let Err(target_index) = self.find(&edge) {
            let rc = Rc::new(RefCell::new(edge));
            self.vec.insert(target_index, rc);
            Some(self.vec[target_index].clone())
        } else {
            None
        }

    }

    /// Remove the given edge from the vec if it exists
    pub fn remove(&mut self, edge: &ScanLineEdge) {
        let result = self.find(edge);
        if let Ok(index) = result {
            self.vec.remove(index);
        }
    }

    /// Returns the index of the first edge that is not less than the input edge
    pub fn lower_bound(&self, edge: &ScanLineEdge) -> usize {
        match self.find(edge) {
            Ok(index) => index,
            Err(index) => index,
        }
    }

    /// Binary search for the edge
    pub fn find(&self, edge: &ScanLineEdge) -> Result<usize, usize> {
        self.vec.binary_search_by(|ptr| ptr.borrow().cmp(edge))
    }

    /// Adds an edge, unordered. For use with [`EdgeVec::left_of`] only.
    pub fn push(&mut self, edge: ScanLineEdge) -> EdgeVecPtr {
        let rc = Rc::new(RefCell::new(edge));
        self.vec.push(rc.clone());
        rc
    }

    /// Removes exactly this edge (by identity, not position).
    pub fn remove_ptr(&mut self, ptr: &EdgeVecPtr) {
        if let Some(i) = self.vec.iter().position(|e| Rc::ptr_eq(e, ptr)) {
            self.vec.swap_remove(i);
        }
    }

    /// The edge immediately to the left of `p` on the sweep line.
    ///
    /// A linear scan with exact predicates. Upstream keeps the edges sorted
    /// and binary searches with a comparator that is not a consistent order
    /// once edges share endpoints or lie on one line, which pixel outlines
    /// do constantly; the search then lands on the wrong edge.
    pub fn left_of(&self, p: &SweepPoint) -> Option<EdgeVecPtr> {
        let mut best: Option<&EdgeVecPtr> = None;
        for ptr in self.vec.iter() {
            if ptr.borrow().side_of(p) >= 0 {
                continue;
            }
            if best.is_none() || ptr.borrow().is_right_of(&best.unwrap().borrow()) {
                best = Some(ptr);
            }
        }
        best.cloned()
    }
}

/// Direction strictly inside the interior wedge at `p` of the chain
/// `prev -> p -> next`. Exact for integer input: a positive combination of
/// the two edge vectors for a convex corner, its negation for a reflex one.
pub fn interior_direction(prev: &PointF64, p: &PointF64, next: &PointF64) -> PointF64 {
    let (a, b) = (*prev - *p, *next - *p);
    let o = orient(prev, p, next);
    if o > 0.0 {
        a + b
    } else if o < 0.0 {
        PointF64::new(-(a.x + b.x), -(a.y + b.y))
    } else if a.dot(b) < 0.0 {
        // Straight: a quarter turn from the outgoing edge, interior side.
        PointF64::new(-b.y, b.x)
    } else {
        a + b
    }
}

/// Splits every edge that passes through another vertex (a T-junction) by
/// inserting a copy of that vertex, so touching happens only at vertices.
pub fn split_t_junctions(rings: &[Vec<PointF64>]) -> Vec<Vec<PointF64>> {
    // Points sorted by x and by y: each edge scans only the points within its
    // narrower extent, which for pixel outlines is a single row or column.
    let mut by_x: Vec<PointF64> = rings.iter().flatten().copied().collect();
    let mut by_y = by_x.clone();
    by_x.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    by_y.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    rings
        .iter()
        .map(|ring| {
            let n = ring.len();
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let (u, w) = (ring[i], ring[(i + 1) % n]);
                out.push(u);
                let (lo_x, hi_x) = (u.x.min(w.x), u.x.max(w.x));
                let (lo_y, hi_y) = (u.y.min(w.y), u.y.max(w.y));
                let candidates = if hi_x - lo_x <= hi_y - lo_y {
                    let a = by_x.partition_point(|p| p.x < lo_x);
                    let b = by_x.partition_point(|p| p.x <= hi_x);
                    &by_x[a..b]
                } else {
                    let a = by_y.partition_point(|p| p.y < lo_y);
                    let b = by_y.partition_point(|p| p.y <= hi_y);
                    &by_y[a..b]
                };
                let mut on: Vec<PointF64> = candidates
                    .iter()
                    .filter(|p| p.x >= lo_x && p.x <= hi_x && p.y >= lo_y && p.y <= hi_y)
                    .filter(|p| is_strictly_between(&u, &w, p))
                    .copied()
                    .collect();
                if on.is_empty() {
                    continue;
                }
                let dir = w - u;
                on.sort_by(|a, b| (*a - u).dot(dir).total_cmp(&(*b - u).dot(dir)));
                on.dedup();
                out.extend(on);
            }
            out
        })
        .collect()
}

/// Where several boundary passes go through one point, re-pairs their
/// incoming and outgoing edges so that each pass bounds exactly one sector of
/// the actual region there, and sets each copy's perturbation into its sector.
///
/// A hole touching the inside of an outline's concave corner is the typical
/// case: each ring's own wedge at the corner covers the other's, and no
/// perturbation of the two copies can separate them without crossing.
/// Re-paired, they become an ordinary pinch with disjoint sectors. Groups
/// that are not a consistent alternation of in/out edges (overlapping
/// edges, self-crossing input) are left alone.
pub fn resolve_shared_points(vertices: &mut [MonotoneVertex], num_vertices: usize) {
    let mut order: Vec<usize> = (0..num_vertices).collect();
    order.sort_by(|&a, &b| {
        let (pa, pb) = (vertices[a].p, vertices[b].p);
        pa.y.total_cmp(&pb.y).then(pa.x.total_cmp(&pb.x))
    });
    let mut start = 0;
    while start < order.len() {
        let mut end = start + 1;
        while end < order.len() && vertices[order[end]].p == vertices[order[start]].p {
            end += 1;
        }
        if end - start >= 2 {
            resolve_group(vertices, &order[start..end]);
        }
        start = end;
    }
}

fn resolve_group(vertices: &mut [MonotoneVertex], group: &[usize]) {
    let x = vertices[group[0]].p;
    let passes: Vec<(usize, PointF64, PointF64)> = group
        .iter()
        .map(|&v| (v, vertices[vertices[v].previous].p, vertices[vertices[v].next].p))
        .collect();
    let pairs = match repair_pinch(&x, &passes) {
        Some(pairs) => pairs,
        None => return,
    };
    let old_next: Vec<(usize, usize)> = group.iter().map(|&v| (v, vertices[v].next)).collect();
    let next_of = |v: usize| old_next.iter().find(|(u, _)| *u == v).unwrap().1;
    let relinks: Vec<(usize, usize)> = pairs.iter().map(|&(vin, vout)| (vin, next_of(vout))).collect();
    for (vin, next) in relinks {
        vertices[vin].next = next;
        vertices[next].previous = vin;
    }
    for &v in group {
        let (prev, next) = (vertices[vertices[v].previous].p, vertices[vertices[v].next].p);
        vertices[v].d = interior_direction(&prev, &x, &next);
    }
}
