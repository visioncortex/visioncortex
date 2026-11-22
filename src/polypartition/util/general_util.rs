use crate::PointF64;

pub fn f64_approximately(a: f64, b: f64) -> bool {
    let epsilon = 1e-7;
    (a - b).abs() <= epsilon
}

pub fn point_f64_approximately(p1: PointF64, p2: PointF64) -> bool {
    f64_approximately(p1.x, p2.x) && f64_approximately(p1.y, p2.y)
}

/// Twice the signed area of triangle (p1, p2, p3). Positive when p3 lies on
/// the convex side of the directed line p1 -> p2 for a counter-clockwise
/// polygon (in this module's sense, see `PolygonProps::get_orientation`),
/// zero when the three points are collinear.
///
/// Exact for integer coordinates up to about 2^25, which covers anything
/// traced from an image; the predicates below rely on the zero case being
/// reported faithfully.
pub fn orient(p1: &PointF64, p2: &PointF64, p3: &PointF64) -> f64 {
    (p3.y - p1.y) * (p2.x - p1.x) - (p3.x - p1.x) * (p2.y - p1.y)
}

/// Strictly convex: collinear points are neither convex nor reflex.
pub fn is_convex(p1: &PointF64, p2: &PointF64, p3: &PointF64) -> bool {
    orient(p1, p2, p3) > 0.0
}

/// Strictly reflex: collinear points are neither convex nor reflex.
pub fn is_reflex(p1: &PointF64, p2: &PointF64, p3: &PointF64) -> bool {
    orient(p1, p2, p3) < 0.0
}

/// `p` lies strictly inside triangle (p1, p2, p3), or on one of its edges.
pub fn is_inside(p1: &PointF64, p2: &PointF64, p3: &PointF64, p: &PointF64) -> bool {
    !(is_convex(p1, p, p2) || is_convex(p2, p, p3) || is_convex(p3, p, p1))
}

/// `p` lies strictly inside the interior wedge at `p2` of a counter-clockwise
/// chain p1 -> p2 -> p3. Points on either bounding ray are outside, so a
/// diagonal running along an edge is never taken to be inside the polygon.
pub fn is_in_cone(p1: &PointF64, p2: &PointF64, p3: &PointF64, p: &PointF64) -> bool {
    if is_convex(p1, p2, p3) {
        is_convex(p1, p2, p) && is_convex(p2, p3, p)
    } else {
        is_convex(p1, p2, p) || is_convex(p2, p3, p)
    }
}

/// `p` is collinear with `a`, `b` and lies strictly between them.
pub fn is_strictly_between(a: &PointF64, b: &PointF64, p: &PointF64) -> bool {
    if orient(a, b, p) != 0.0 {
        return false;
    }
    let ab = *b - *a;
    (*p - *a).dot(ab) > 0.0 && (*b - *p).dot(ab) > 0.0
}

/// Whether the closed segment `e1`-`e2` touches the open segment `d1`-`d2`:
/// a proper crossing, an endpoint of `e` on the open segment, or a collinear
/// overlap. Touching only at `d1` or `d2` does not count.
///
/// This is the visibility test for a candidate diagonal or bridge `d1`-`d2`:
/// unlike `intersects`, a diagonal passing exactly through a vertex is
/// blocked, and an edge that is merely collinear with it but elsewhere on the
/// line is not.
pub fn segment_blocks(d1: &PointF64, d2: &PointF64, e1: &PointF64, e2: &PointF64) -> bool {
    let o1 = orient(d1, d2, e1);
    let o2 = orient(d1, d2, e2);
    if (o1 > 0.0 && o2 > 0.0) || (o1 < 0.0 && o2 < 0.0) {
        return false;
    }
    if o1 == 0.0 && o2 == 0.0 {
        // Collinear: blocked if the two overlap in more than a point.
        let d = *d2 - *d1;
        let (t1, t2) = ((*e1 - *d1).dot(d), (*e2 - *d1).dot(d));
        let (lo, hi) = (t1.min(t2), t1.max(t2));
        return lo < d.dot(d) && hi > 0.0;
    }
    if is_strictly_between(d1, d2, e1) || is_strictly_between(d1, d2, e2) {
        return true;
    }
    let o3 = orient(e1, e2, d1);
    let o4 = orient(e1, e2, d2);
    ((o1 > 0.0 && o2 < 0.0) || (o1 < 0.0 && o2 > 0.0))
        && ((o3 > 0.0 && o4 < 0.0) || (o3 < 0.0 && o4 > 0.0))
}

/// Drops repeated points and the tips of zero-width spikes (a vertex whose
/// neighbours lie on the same ray from it), repeatedly, around the ring.
/// Neither encloses any area, and no non-degenerate triangle can contain
/// a spike's tip.
pub fn remove_degenerate_points(points: &[PointF64]) -> Vec<PointF64> {
    let is_degenerate = |a: &PointF64, b: &PointF64, c: &PointF64| {
        a == b || b == c || (orient(a, b, c) == 0.0 && (*a - *b).dot(*c - *b) > 0.0)
    };
    let mut ring: Vec<PointF64> = points.to_vec();
    loop {
        let n = ring.len();
        if n < 3 {
            return ring;
        }
        match (0..n).find(|&i| is_degenerate(&ring[(i + n - 1) % n], &ring[i], &ring[(i + 1) % n])) {
            Some(i) => {
                ring.remove(i);
            }
            None => return ring,
        }
    }
}

/// Re-pairs the boundary passes through one shared point `x` so that each
/// bounds exactly one sector of the region there. `passes` holds, per pass,
/// an id and the points before and after it. Returns `(a, b)` pairs: pass `a`
/// keeps its incoming edge and takes over the outgoing edge of pass `b`.
///
/// Where a hole touches the inside of an outline's concave corner, or two
/// holes touch, each pass's own wedge covers the other's. Re-paired, they
/// become an ordinary pinch whose sectors do not overlap. Returns `None`
/// unless incoming and outgoing edges alternate around `x` (overlapping
/// edges, crossing input).
pub fn repair_pinch(x: &PointF64, passes: &[(usize, PointF64, PointF64)]) -> Option<Vec<(usize, usize)>> {
    let cross = |u: PointF64, w: PointF64| u.x * w.y - u.y * w.x;
    // (direction, is_outgoing, pass)
    let mut rays: Vec<(PointF64, bool, usize)> = Vec::with_capacity(passes.len() * 2);
    for &(id, prev, next) in passes {
        rays.push((prev - *x, false, id));
        rays.push((next - *x, true, id));
    }
    if rays.iter().any(|r| r.0.x == 0.0 && r.0.y == 0.0) {
        return None;
    }
    // Counter-clockwise in the y-down plane is increasing atan2(y, x); a
    // pass's interior runs from its outgoing edge that way to its incoming.
    // An incoming and an outgoing edge along the same ray are a hole bridge,
    // a zero-width slit with the region on both sides: the incoming edge
    // comes first.
    let half = |d: &PointF64| if d.y > 0.0 || (d.y == 0.0 && d.x > 0.0) { 0 } else { 1 };
    rays.sort_by(|a, b| {
        half(&a.0)
            .cmp(&half(&b.0))
            .then_with(|| 0.0_f64.total_cmp(&cross(a.0, b.0)))
            .then_with(|| a.1.cmp(&b.1))
    });
    let m = rays.len();
    let mut pairs = Vec::with_capacity(passes.len());
    for k in 0..m {
        let (a, b) = (rays[k], rays[(k + 1) % m]);
        if cross(a.0, b.0) == 0.0 && a.0.dot(b.0) > 0.0 && a.1 == b.1 {
            return None; // two edges overlap in the same direction
        }
        if a.1 {
            if b.1 {
                return None;
            }
            pairs.push((b.2, a.2));
        }
    }
    Some(pairs)
}

/// Splits a weakly simple ring wherever it passes through a point more than
/// once with overlapping wedges (see [`repair_pinch`]), returning rings that
/// together cover the same region and meet only at clean pinches. A ring
/// without such points comes back unchanged.
pub fn resolve_pinches(points: &[PointF64]) -> Vec<Vec<PointF64>> {
    let n = points.len();
    if n < 3 {
        return vec![points.to_vec()];
    }
    let mut next: Vec<usize> = (0..n).map(|i| (i + 1) % n).collect();
    let mut prev: Vec<usize> = (0..n).map(|i| (i + n - 1) % n).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| points[a].y.total_cmp(&points[b].y).then(points[a].x.total_cmp(&points[b].x)));
    let mut changed = false;
    let mut start = 0;
    while start < n {
        let mut end = start + 1;
        while end < n && points[order[end]] == points[order[start]] {
            end += 1;
        }
        if end - start >= 2 {
            let group = &order[start..end];
            let passes: Vec<(usize, PointF64, PointF64)> =
                group.iter().map(|&v| (v, points[prev[v]], points[next[v]])).collect();
            if let Some(pairs) = repair_pinch(&points[order[start]], &passes) {
                let old_next: Vec<(usize, usize)> = group.iter().map(|&v| (v, next[v])).collect();
                for (vin, vout) in pairs {
                    let target = old_next.iter().find(|(u, _)| *u == vout).unwrap().1;
                    if next[vin] != target {
                        next[vin] = target;
                        prev[target] = vin;
                        changed = true;
                    }
                }
            }
        }
        start = end;
    }
    if !changed {
        return vec![points.to_vec()];
    }
    let mut seen = vec![false; n];
    let mut rings = vec![];
    for s in 0..n {
        if seen[s] {
            continue;
        }
        let mut ring = vec![];
        let mut v = s;
        while !seen[v] {
            seen[v] = true;
            ring.push(points[v]);
            v = next[v];
        }
        rings.push(ring);
    }
    rings
}

pub fn normalize(p: &PointF64) -> PointF64 {
    let norm = p.norm();
    if norm != 0.0 {
        PointF64::new(p.x / norm, p.y / norm)
    } else {
        PointF64::new(0.0, 0.0)
    }
}

pub fn distance(p1: &PointF64, p2: &PointF64) -> f64 {
    (*p1 - *p2).norm()
}

pub fn intersects(p11: &PointF64, p12: &PointF64, p21: &PointF64, p22: &PointF64) -> bool {
    let (p11, p12, p21, p22) = (*p11, *p12, *p21, *p22);

    if p11 == p21 || p11 == p22 || p12 == p21 || p12 == p22 {
        return false;
    }
    
    let v1ort = PointF64::new(p12.y - p11.y, p11.x - p12.x);
    let v2ort = PointF64::new(p22.y - p21.y, p21.x - p22.x);

    let v = p21 - p11;
    let dot21 = v.dot(v1ort);

    let v = p22 - p11;
    let dot22 = v.dot(v1ort);

    let v = p11 - p21;
    let dot11 = v.dot(v2ort);

    let v = p12 - p21;
    let dot12 = v.dot(v2ort);

    !(dot11 * dot12 > 0.0 || dot21 * dot22 > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn util_general_point() {
        let p = PointF64::new(1.0, 0.0);
        assert!(normalize(&p) == p);
        let p = PointF64::new(1.0, 1.0);
        assert!(point_f64_approximately(normalize(&p), PointF64::new(1.0 / 2.0_f64.sqrt(), 1.0 / 2.0_f64.sqrt())));
        let p = PointF64::new(3.0, 3.0);
        assert!(point_f64_approximately(normalize(&p), PointF64::new(1.0 / 2.0_f64.sqrt(), 1.0 / 2.0_f64.sqrt())));
    }

    #[test]
    fn util_general_is_inside() {
        let p1 = &PointF64::new(-1.0, -1.0);
        let p2 = &PointF64::new(1.0, -1.0);
        let p3 = &PointF64::new(0.0, 1.0);
        let p = &PointF64::new(0.0, 0.5);

        assert!(is_inside(p1, p2, p3, p));

        let p1 = &PointF64::new(-1.0, -1.0);
        let p2 = &PointF64::new(1.0, -1.0);
        let p3 = &PointF64::new(0.0, 1.0);
        let p = &PointF64::new(0.0, 9.0);

        assert!(!is_inside(p1, p2, p3, p));
    }

    #[test]
    fn util_general_intersects() {
        let p11 = &PointF64::new(-1.0, 0.0);
        let p12 = &PointF64::new(1.0, 0.0);
        let p21 = &PointF64::new(0.0, -1.0);
        let p22 = &PointF64::new(0.0, 1.0);

        assert!(intersects(p11, p12, p21, p22));

        let p11 = &PointF64::new(-1.0, 0.0);
        let p12 = &PointF64::new(1.0, 0.0);
        let p21 = &PointF64::new(-1.0, -1.0);
        let p22 = &PointF64::new(1.0, -1.0);

        assert!(!intersects(p11, p12, p21, p22));
    }
}
