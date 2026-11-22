use crate::polypartition::{PartitionVertex, PartitionVertexInfo, Polygon, PolygonInterface};
use crate::polypartition::util::{is_degenerate_vertex, orient, resolve_pinches, update_vertex};

pub fn triangulate_ec_vec(polys: Vec<Polygon>) -> Result<Vec<Polygon>, String> {
    let mut triangles = vec![];
    for poly in polys.iter() {
        triangles.extend(triangulate_ec(poly)?);
    }
    Ok(triangles)
}

/// Takes an arbitrary polygon.
///
/// Returns a vec of triangles.
pub fn triangulate_ec(poly: &Polygon) -> Result<Vec<Polygon>, &str> {
    triangulate_ec_static(poly)
}

fn triangulate_ec_static(poly: &Polygon) -> Result<Vec<Polygon>, &'static str> {
    if !poly.is_valid() {
        return Err("Input polygon is invalid.");
    }

    let mut triangles = vec![];
    let num_vertices = poly.num_points();

    // Just in case (should be guarded above)
    if num_vertices < 3 {
        return Err("Input polygon has less than 3 vertices");
    }
    // Trivial case
    if num_vertices == 3 {
        return Ok(vec![poly.clone()]);
    }

    // Where the ring passes through a point twice with overlapping wedges
    // (holes bridged in separately that touch each other or the outline),
    // no ear test based on the ring's own local geometry is sound. Split it
    // there into rings that only meet at clean pinches.
    let rings = resolve_pinches(&poly.props().points);
    if rings.len() > 1 {
        for ring in rings {
            if ring.len() >= 3 {
                triangles.extend(triangulate_ec_static(&Polygon::from_points_and_is_hole(ring, false))?);
            }
        }
        return Ok(triangles);
    }

    let mut vertices = vec![PartitionVertex::default(); num_vertices];

    for (i, vertex) in vertices.iter_mut().enumerate() {
        let mut info = PartitionVertexInfo::default();
        // Fill in info
        info.is_active = true;
        info.p = poly.get_point(i);

        vertex.set_info(info);

        // Setting previous/next
        if i == (num_vertices - 1) { // Last
            vertex.next = 0; // Wrap around
        } else {
            vertex.next = i+1; // Just the next
        }
        if i == 0 { // First
            vertex.previous = num_vertices - 1; // Wrap around
        } else {
            vertex.previous= i-1; // Just the previous
        }
    }
    let mut num_active = num_vertices;

    // Repeated points and zero-width spikes enclose nothing; drop them first
    // so they can neither become ears nor block real ones.
    for i in 0..num_vertices {
        prune_degenerate(i, &mut vertices, &mut num_active);
    }

    // Update the angles and is_ear the first time
    for i in 0..num_vertices {
        if vertices[i].info.is_active {
            update_vertex(i, &mut vertices);
        }
    }

    while num_active > 3 {
        // Find optimal ear
        let mut ear = find_ear(&vertices);
        if ear.is_none() {
            // Ear flags are only refreshed next to a clipped ear, so one may
            // still be blocked by a vertex that has since gone.
            for i in 0..num_vertices {
                if vertices[i].info.is_active {
                    update_vertex(i, &mut vertices);
                }
            }
            ear = find_ear(&vertices);
        }
        let ear = match ear {
            Some(ear) => ear,
            None => {
                // A straight vertex is never an ear, but removing it does not
                // change the polygon and may unblock one.
                let straight = (0..num_vertices).find(|&i| {
                    let v = &vertices[i];
                    v.info.is_active
                        && orient(&vertices[v.previous].info.p, &v.info.p, &vertices[v.next].info.p) == 0.0
                });
                match straight {
                    Some(v) => {
                        let (prev, next) = unlink(v, &mut vertices, &mut num_active);
                        refresh(prev, next, &mut vertices, &mut num_active);
                        continue;
                    }
                    None => return Err("No ear found!"),
                }
            }
        };
        let prev = vertices[ear].previous;
        let next = vertices[ear].next;

        triangles.push(Polygon::triangle(
            vertices[prev].info.p,
            vertices[ear].info.p,
            vertices[next].info.p,
        ));

        unlink(ear, &mut vertices, &mut num_active);

        if num_active == 3 {
            break;
        }

        refresh(prev, next, &mut vertices, &mut num_active);
    }

    if num_active == 3 {
        for vertex in vertices.iter() {
            if vertex.info.is_active {
                let (a, b, c) = (vertices[vertex.previous].info.p, vertex.info.p, vertices[vertex.next].info.p);
                if orient(&a, &b, &c) != 0.0 {
                    triangles.push(Polygon::triangle(a, b, c));
                }
                break;
            }
        }
    }

    Ok(triangles)
}

/// The convex ear with the sharpest angle; the first such on ties.
fn find_ear(vertices: &[PartitionVertex]) -> Option<usize> {
    let mut optimal_ear: Option<usize> = None;
    for (i, vertex) in vertices.iter().enumerate() {
        if !vertex.info.is_active || !vertex.info.is_ear {
            continue;
        }
        match optimal_ear {
            Some(o) if vertex.info.angle <= vertices[o].info.angle => {}
            _ => optimal_ear = Some(i),
        }
    }
    optimal_ear
}

/// Removes `v` from the ring; returns its former neighbours.
fn unlink(v: usize, vertices: &mut [PartitionVertex], num_active: &mut usize) -> (usize, usize) {
    let prev = vertices[v].previous;
    let next = vertices[v].next;
    vertices[v].info.is_active = false;
    // Tighten the loose ends
    vertices[prev].next = next;
    vertices[next].previous = prev;
    *num_active -= 1;
    (prev, next)
}

/// Drops `v` and then its neighbours, transitively, while they are degenerate.
/// Returns the vertices examined that are still in the ring: the new seam.
fn prune_degenerate(v: usize, vertices: &mut [PartitionVertex], num_active: &mut usize) -> Vec<usize> {
    let mut seam = vec![];
    let mut stack = vec![v];
    while let Some(v) = stack.pop() {
        if !vertices[v].info.is_active {
            continue;
        }
        if *num_active <= 2 || !is_degenerate_vertex(v, vertices) {
            seam.push(v);
            continue;
        }
        let (prev, next) = unlink(v, vertices, num_active);
        stack.push(prev);
        stack.push(next);
    }
    seam
}

/// Re-evaluates the vertices that just became adjacent.
fn refresh(prev: usize, next: usize, vertices: &mut Vec<PartitionVertex>, num_active: &mut usize) {
    let mut seam = prune_degenerate(prev, vertices, num_active);
    seam.extend(prune_degenerate(next, vertices, num_active));
    if *num_active < 3 {
        return;
    }
    for v in seam {
        if vertices[v].info.is_active {
            update_vertex(v, vertices);
        }
    }
}
