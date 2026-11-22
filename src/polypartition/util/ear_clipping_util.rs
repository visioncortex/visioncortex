use crate::polypartition::PartitionVertex;
use crate::PointF64;

use super::{is_reflex, is_strictly_between, normalize, orient, point_f64_approximately};

pub fn update_vertex_reflexity(v: usize, vertices: &mut Vec<PartitionVertex>) {
    let v1 = vertices[v].previous;
    let v3 = vertices[v].next;
    let v1_info = &vertices[v1].info;
    let v3_info = &vertices[v3].info;
    vertices[v].info.is_convex = !is_reflex(&v1_info.p, &vertices[v].info.p, &v3_info.p);
}

/// Recomputes convexity, angle and ear status of `v` against the vertices
/// still in the polygon.
///
/// `v` is an ear when it is strictly convex and no other remaining vertex
/// - lies strictly inside the triangle (prev, v, next),
/// - lies on its open diagonal prev-next: a vertex there (e.g. the reflex
///   corner of an L traced from pixels) would otherwise be clipped across,
///   overlapping the triangles cut later, or
/// - lies on one of the other two sides with an edge leading into the
///   triangle: a T-junction where, say, a hole corner touches the outline.
///
/// Coincident copies of the three corners (pinch points, hole bridges) block
/// only if one of their edges leads into the triangle. Upstream skips them
/// outright, which is wrong where a hole touches the outline at a concave
/// corner: the two passes through that point have overlapping wedges.
pub fn update_vertex(v: usize, vertices: &mut Vec<PartitionVertex>) {
    let v1 = vertices[v].previous;
    let v3 = vertices[v].next;
    let a = vertices[v1].info.p;
    let b = vertices[v].info.p;
    let c = vertices[v3].info.p;
    vertices[v].info.is_convex = orient(&a, &b, &c) > 0.0;

    let vec1 = normalize(&(a - b));
    let vec3 = normalize(&(c - b));
    vertices[v].info.angle = vec1.x * vec3.x + vec1.y * vec3.y;

    if !vertices[v].info.is_convex {
        vertices[v].info.is_ear = false;
        return;
    }
    let blocks = |vertex: &PartitionVertex| {
        if !vertex.info.is_active {
            return false;
        }
        let p = vertex.info.p;
        let (prev, next) = (vertices[vertex.previous].info.p, vertices[vertex.next].info.p);
        // A vertex sitting on a corner, side or the diagonal blocks when one
        // of its edges heads into the triangle. Both lines must pass through
        // the vertex for the test to be about the edge's direction.
        let enters = |l1: (&PointF64, &PointF64), l2: (&PointF64, &PointF64)| {
            [prev, next].iter().any(|q| orient(l1.0, l1.1, q) > 0.0 && orient(l2.0, l2.1, q) > 0.0)
        };
        if point_f64_approximately(p, a) {
            return enters((&a, &b), (&c, &a));
        }
        if point_f64_approximately(p, b) {
            return enters((&a, &b), (&b, &c));
        }
        if point_f64_approximately(p, c) {
            return enters((&b, &c), (&c, &a));
        }
        let (oab, obc, oca) = (orient(&a, &b, &p), orient(&b, &c, &p), orient(&c, &a, &p));
        if oab > 0.0 && obc > 0.0 && oca > 0.0 {
            return true;
        }
        if is_strictly_between(&c, &a, &p) {
            return true;
        }
        let on_side_entering = |s1: &PointF64, s2: &PointF64| {
            is_strictly_between(s1, s2, &p) && (orient(s1, s2, &prev) > 0.0 || orient(s1, s2, &next) > 0.0)
        };
        on_side_entering(&a, &b) || on_side_entering(&b, &c)
    };
    // An edge passing straight through a corner's position (a T-junction
    // there) can cross the triangle without any of its vertices touching it.
    let crosses_corner = |vertex: &PartitionVertex| {
        if !vertex.info.is_active {
            return false;
        }
        let (u, w) = (vertex.info.p, vertices[vertex.next].info.p);
        let into = |l1: (&PointF64, &PointF64), l2: (&PointF64, &PointF64)| {
            [u, w].iter().any(|q| orient(l1.0, l1.1, q) > 0.0 && orient(l2.0, l2.1, q) > 0.0)
        };
        (is_strictly_between(&u, &w, &a) && into((&a, &b), (&c, &a)))
            || (is_strictly_between(&u, &w, &b) && into((&a, &b), (&b, &c)))
            || (is_strictly_between(&u, &w, &c) && into((&b, &c), (&c, &a)))
    };
    if vertices.iter().any(|vertex| blocks(vertex) || crosses_corner(vertex)) {
        vertices[v].info.is_ear = false;
        return;
    }
    // Nothing crosses the open triangle, so it lies in a single face of the
    // ring. Usually that is the interior, but clipping can leave zero-width
    // doubled corridors whose both sides are outside, and a bend in one looks
    // like a perfectly good ear locally.
    let centroid = PointF64::new((a.x + b.x + c.x) / 3.0, (a.y + b.y + c.y) / 3.0);
    vertices[v].info.is_ear = is_inside_ring(&centroid, vertices);
}

/// Even-odd containment in the ring of active vertices.
fn is_inside_ring(p: &PointF64, vertices: &[PartitionVertex]) -> bool {
    let mut inside = false;
    for vertex in vertices.iter().filter(|vertex| vertex.info.is_active) {
        let (a, b) = (vertex.info.p, vertices[vertex.next].info.p);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
            inside = !inside;
        }
    }
    inside
}

/// A vertex that encloses no area with its neighbours and points back along
/// the edge it came in on: a repeated point, or the tip of a zero-width
/// spike. Removing it leaves the polygon's area and interior unchanged.
pub fn is_degenerate_vertex(v: usize, vertices: &[PartitionVertex]) -> bool {
    let a = vertices[vertices[v].previous].info.p;
    let b = vertices[v].info.p;
    let c = vertices[vertices[v].next].info.p;
    a == b || b == c || (orient(&a, &b, &c) == 0.0 && (a - b).dot(c - b) > 0.0)
}
