use super::{Polygon, PolygonInterface, is_in_cone, is_strictly_between, normalize, point_f64_approximately, segment_blocks};
use crate::PointF64;

/// Merges every hole into a non-hole polygon by cutting a zero-width bridge
/// from the hole's rightmost point to a visible vertex of the polygon.
///
/// The result is weakly simple: bridge edges are traversed twice, and a hole
/// that touches the outline at a vertex is spliced in at that vertex.
pub fn remove_holes(inpolys: &[Polygon]) -> Result<Vec<Polygon>, &str> {
    // Check for the trivial case of no holes
    if !inpolys.iter().any(|polygon| polygon.is_hole()) {
        return Ok(inpolys.to_vec());
    }

    let mut polys = inpolys.to_vec();
    // Repeatedly merge hole polygon with a suitable non-hole polygon
    // until no hole polygons are present.
    // Each time look for the hole polygon with the largest x
    while let Some((hole_polygon_index, holepoint_index)) = rightmost_hole(&polys) {
        let hole_polygon = &polys[hole_polygon_index];
        let hole_num_points = hole_polygon.num_points();
        let holepoint = hole_polygon.get_point(holepoint_index);
        let hole_prev = hole_polygon.get_point((holepoint_index + hole_num_points - 1) % hole_num_points);
        let hole_next = hole_polygon.get_point((holepoint_index + 1) % hole_num_points);

        // Upstream's search: the vertex to the right of the hole whose
        // direction from the hole point is closest to +x, among those visible.
        let mut best: Option<(usize, usize)> = None;
        for (polygon_index, polygon) in polys.iter().enumerate() {
            if polygon.is_hole() {
                continue;
            }
            let points = &polygon.props().points;
            let num_points = points.len();
            for i in 0..num_points {
                if points[i].x <= holepoint.x {
                    continue;
                }
                // Basic check
                let prev = points[(i + num_points - 1) % num_points];
                let next = points[(i + 1) % num_points];
                if !is_in_cone(&prev, &points[i], &next, &holepoint) {
                    continue;
                }
                let polypoint = points[i];
                // Check optimality
                if let Some((bp, bi)) = best {
                    let best_polypoint = polys[bp].get_point(bi);
                    let v1 = normalize(&(polypoint - holepoint));
                    let v2 = normalize(&(best_polypoint - holepoint));
                    if v2.x > v1.x {
                        continue;
                    }
                }
                if is_visible(&polys, &holepoint, &polypoint) {
                    best = Some((polygon_index, i));
                }
            }
        }

        // Pixel holes often touch the outline at a corner, which can also be
        // the hole's rightmost point, leaving nothing strictly to the right.
        // Splice the hole in at the shared vertex instead, or, if the hole
        // point sits on an edge, at a vertex inserted there.
        let mut join = Join::Bridge;
        if best.is_none() {
            best = find_touching(&polys, &holepoint, &hole_prev, &hole_next);
            join = Join::Splice;
        }
        if best.is_none() {
            best = find_touching_edge(&polys, &holepoint, &hole_prev, &hole_next);
            join = Join::Insert;
        }
        if best.is_none() {
            join = Join::Bridge;
        }
        // Last resort: any visible vertex whose bridge leaves both the hole
        // and the polygon vertex on their interior side.
        if best.is_none() {
            best = find_nearest_visible(&polys, &holepoint, &hole_prev, &hole_next);
        }

        let (best_polygon_index, polypoint_index) = match best {
            Some(best) => best,
            None => return Err("No visible polypoint found. Cannot merge hole polygon with a non-hole polygon."),
        };
        let best_polygon = &polys[best_polygon_index];

        // Construct the new polygon (merging hole_polygon and best_polygon)
        let mut newpoly_points = Vec::with_capacity(
            hole_num_points + best_polygon.num_points() + 2
        );
        // Insert the points in non-hole polygon up until the polypoint
        for i in 0..=polypoint_index {
            newpoly_points.push(best_polygon.get_point(i));
        }
        match join {
            Join::Splice => {
                // The hole point is the polypoint: walk the hole once, ending on it.
                for i in 1..=hole_num_points {
                    newpoly_points.push(hole_polygon.get_point((i + holepoint_index) % hole_num_points));
                }
                for i in (polypoint_index + 1)..best_polygon.num_points() {
                    newpoly_points.push(best_polygon.get_point(i));
                }
            }
            Join::Insert => {
                // The hole point splits the edge after the polypoint.
                for i in 0..=hole_num_points {
                    newpoly_points.push(hole_polygon.get_point((i + holepoint_index) % hole_num_points));
                }
                for i in (polypoint_index + 1)..best_polygon.num_points() {
                    newpoly_points.push(best_polygon.get_point(i));
                }
            }
            Join::Bridge => {
                // Insert all points in hole polygon in a cyclic manner STARTING from holepoint
                for i in 0..=hole_num_points {
                    newpoly_points.push(hole_polygon.get_point((i + holepoint_index) % hole_num_points));
                }
                // Insert the rest of the points in non-hole polygon
                for i in polypoint_index..best_polygon.num_points() {
                    newpoly_points.push(best_polygon.get_point(i));
                }
            }
        }
        let newpoly = Polygon::from_points_and_is_hole(newpoly_points, false);

        // Remove the higher index first so the other stays valid.
        polys.remove(hole_polygon_index.max(best_polygon_index));
        polys.remove(hole_polygon_index.min(best_polygon_index));
        polys.push(newpoly);
    }

    Ok(polys)
}

enum Join {
    /// A two-way bridge from the hole point to the polypoint.
    Bridge,
    /// The hole point coincides with the polypoint.
    Splice,
    /// The hole point lies on the edge leaving the polypoint.
    Insert,
}

/// The hole with the largest x, and the index of its (first) rightmost point.
fn rightmost_hole(polys: &[Polygon]) -> Option<(usize, usize)> {
    let mut found: Option<(usize, usize, f64)> = None;
    for (polygon_index, polygon) in polys.iter().enumerate() {
        if !polygon.is_hole() || polygon.num_points() == 0 {
            continue;
        }
        let mut holepoint_index = 0;
        for (i, p) in polygon.props().points.iter().enumerate().skip(1) {
            if p.x > polygon.get_point(holepoint_index).x {
                holepoint_index = i;
            }
        }
        let x = polygon.get_point(holepoint_index).x;
        if !matches!(found, Some((_, _, best_x)) if x <= best_x) {
            found = Some((polygon_index, holepoint_index, x));
        }
    }
    found.map(|(polygon_index, holepoint_index, _)| (polygon_index, holepoint_index))
}

/// No edge of any polygon, hole or not, touches the open bridge, and the
/// bridge runs through the region rather than through a hole.
///
/// The second half matters when both ends are vertices of the same ring: a
/// bridge between two corners of an already merged pixel hole touches no
/// edge, yet lies entirely inside that hole.
fn is_visible(polys: &[Polygon], holepoint: &PointF64, polypoint: &PointF64) -> bool {
    let clear = polys.iter().all(|poly| {
        let points = &poly.props().points;
        let num_points = points.len();
        (0..num_points).all(|i| {
            !segment_blocks(holepoint, polypoint, &points[i], &points[(i + 1) % num_points])
        })
    });
    // With nothing touching the open segment, it lies in a single face, and
    // its midpoint tells which.
    clear && is_in_region(polys, &((*holepoint + *polypoint) * 0.5))
}

/// Even-odd point-in-region over every ring; doubled bridge edges cancel.
fn is_in_region(polys: &[Polygon], p: &PointF64) -> bool {
    let mut inside = false;
    for poly in polys {
        let points = &poly.props().points;
        let n = points.len();
        for i in 0..n {
            let (a, b) = (points[i], points[(i + 1) % n]);
            if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
                inside = !inside;
            }
        }
    }
    inside
}

/// A non-hole vertex at the hole point whose interior wedge holds the hole.
fn find_touching(
    polys: &[Polygon], holepoint: &PointF64, hole_prev: &PointF64, hole_next: &PointF64,
) -> Option<(usize, usize)> {
    for (polygon_index, polygon) in polys.iter().enumerate() {
        if polygon.is_hole() {
            continue;
        }
        let points = &polygon.props().points;
        let num_points = points.len();
        for i in 0..num_points {
            if !point_f64_approximately(points[i], *holepoint) {
                continue;
            }
            let prev = points[(i + num_points - 1) % num_points];
            let next = points[(i + 1) % num_points];
            if is_in_cone(&prev, &points[i], &next, hole_prev)
                && is_in_cone(&prev, &points[i], &next, hole_next)
            {
                return Some((polygon_index, i));
            }
        }
    }
    None
}

/// A non-hole edge passing through the hole point, with the hole on its
/// interior side.
fn find_touching_edge(
    polys: &[Polygon], holepoint: &PointF64, hole_prev: &PointF64, hole_next: &PointF64,
) -> Option<(usize, usize)> {
    for (polygon_index, polygon) in polys.iter().enumerate() {
        if polygon.is_hole() {
            continue;
        }
        let points = &polygon.props().points;
        let num_points = points.len();
        for i in 0..num_points {
            let (u, w) = (points[i], points[(i + 1) % num_points]);
            if is_strictly_between(&u, &w, holepoint)
                && is_in_cone(&u, holepoint, &w, hole_prev)
                && is_in_cone(&u, holepoint, &w, hole_next)
            {
                return Some((polygon_index, i));
            }
        }
    }
    None
}

fn find_nearest_visible(
    polys: &[Polygon], holepoint: &PointF64, hole_prev: &PointF64, hole_next: &PointF64,
) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize, f64)> = None;
    for (polygon_index, polygon) in polys.iter().enumerate() {
        if polygon.is_hole() {
            continue;
        }
        let points = &polygon.props().points;
        let num_points = points.len();
        for i in 0..num_points {
            let polypoint = points[i];
            if point_f64_approximately(polypoint, *holepoint) {
                continue;
            }
            let d = (polypoint - *holepoint).norm();
            if matches!(best, Some((_, _, best_d)) if d >= best_d) {
                continue;
            }
            let prev = points[(i + num_points - 1) % num_points];
            let next = points[(i + 1) % num_points];
            // The hole is clockwise, so its "cone" is the polygon side.
            if is_in_cone(&prev, &polypoint, &next, holepoint)
                && is_in_cone(hole_prev, holepoint, hole_next, &polypoint)
                && is_visible(polys, holepoint, &polypoint)
            {
                best = Some((polygon_index, i, d));
            }
        }
    }
    best.map(|(polygon_index, i, _)| (polygon_index, i))
}
