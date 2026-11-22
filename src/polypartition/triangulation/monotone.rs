use crate::polypartition::{EdgeVec, Polygon, PolygonInterface};
use crate::polypartition::util::{
    add_diagonal, interior_direction, orient_sign, remove_degenerate_points, resolve_shared_points, split_t_junctions,
    sweep_before, MonotoneVertex, ScanLineEdge, SweepPoint,
};
use crate::polypartition::enums::VertexType;

use crate::PointF64;

pub fn triangulate_mono_vec(polys: Vec<Polygon>) -> Result<Vec<Polygon>, String> {
    let pieces = partition(polys)?;
    let mut triangles = vec![];
    for piece in pieces.iter() {
        triangles.extend(triangulate_monotone_points(piece)?);
    }
    Ok(triangles)
}

/// Takes a polygon monotone in y.
///
/// Returns a vec of triangles.
pub fn triangulate_mono(poly: &Polygon) -> Result<Vec<Polygon>, &str> {
    if !poly.is_valid() {
        return Err("Input polygon is invalid.");
    }

    // Trivial case
    if poly.num_points() == 3 {
        return Ok(vec![poly.clone()]);
    }

    let points = &poly.props().points;
    let n = points.len();
    let mut sweep: Vec<SweepPoint> = points
        .iter()
        .enumerate()
        .map(|(index, &p)| SweepPoint { p, d: PointF64::default(), index })
        .collect();
    // Copies of one point (a pinch) are pulled into their own sectors.
    for i in 0..n {
        if (0..n).any(|j| j != i && points[j] == points[i]) {
            sweep[i].d = interior_direction(&points[(i + n - 1) % n], &points[i], &points[(i + 1) % n]);
        }
    }
    triangulate_monotone_points(&sweep)
}

/// `a` is later in the sweep than `b` (upstream's `Below`).
fn later(a: &SweepPoint, b: &SweepPoint) -> bool {
    sweep_before(b, a)
}

fn triangulate_monotone_points(points: &[SweepPoint]) -> Result<Vec<Polygon>, &'static str> {
    let num_points = points.len();
    if num_points < 3 {
        return Err("Input polygon is invalid.");
    }
    // Trivial case
    if num_points == 3 {
        return Ok(vec![Polygon::triangle(points[0].p, points[1].p, points[2].p)]);
    }
    let convex = |a: usize, b: usize, c: usize| orient_sign(&points[a], &points[b], &points[c]) > 0;

    let mut top_index = 0;
    let mut bottom_index = 0;
    // Find the top-most and bottom-most points
    for i in 1..num_points {
        if !later(&points[i], &points[bottom_index]) {
            bottom_index = i;
        }
        if !later(&points[top_index], &points[i]) {
            top_index = i;
        }
    }

    // Check if the polygon is really monotone
    {
        let mut i;

        i = top_index;
        while i != bottom_index {
            let i2 = (i+1) % num_points;
            if later(&points[i2], &points[i]) {
                return Err("Input polygon is not monotone.");
            }
            i = i2;
        }

        i = bottom_index;
        while i != top_index {
            let i2 = (i+1) % num_points;
            if later(&points[i], &points[i2]) {
                return Err("Input polygon is not monotone.");
            }
            i = i2;
        }
    }

    let mut vertex_types = vec![0_i8; num_points];
    let mut priority = vec![0_usize; num_points];

    // Merge left and right vertex chains
    priority[0] = top_index;
    vertex_types[top_index] = 0;
    let mut left_index = (top_index+1) % num_points;
    let mut right_index = if top_index == 0 {num_points-1} else {top_index-1};
    for p in priority.iter_mut().skip(1).take(num_points - 2) {
        if left_index == bottom_index {
            *p = right_index;
            right_index = if right_index==0 {num_points-1} else {right_index-1};
            vertex_types[*p] = -1;
        } else if right_index == bottom_index {
            *p = left_index;
            left_index = (left_index+1) % num_points;
            vertex_types[*p] = 1;
        } else if !later(&points[left_index], &points[right_index]) {
            *p = right_index;
            right_index = if right_index==0 {num_points-1} else {right_index-1};
            vertex_types[*p]  = -1;
        } else {
            *p = left_index;
            left_index = (left_index+1) % num_points;
            vertex_types[*p] = 1;
        }
    }
    priority[num_points-1] = bottom_index;
    vertex_types[bottom_index] = 0;

    let mut stack = vec![0_usize; num_points];
    let mut stack_ptr = 2;

    stack[0] = priority[0];
    stack[1] = priority[1];

    let mut triangles = vec![];
    let tri = |a: usize, b: usize, c: usize| Polygon::triangle(points[a].p, points[b].p, points[c].p);

    // For each vertex from top to bottom trim as many triangles as possible
    for i in 2..(num_points-1) {
        let v_index = priority[i];
        if vertex_types[v_index] != vertex_types[stack[stack_ptr - 1]] {
            for j in 0..(stack_ptr-1) {
                if vertex_types[v_index] == 1 {
                    triangles.push(tri(stack[j+1], stack[j], v_index));
                } else {
                    triangles.push(tri(stack[j], stack[j+1], v_index));
                }
            }
            stack[0] = priority[i-1];
            stack[1] = priority[i];
            stack_ptr = 2;
        } else {
            stack_ptr -= 1;
            while stack_ptr > 0 {
                if vertex_types[v_index] == 1 {
                    if convex(v_index, stack[stack_ptr - 1], stack[stack_ptr]) {
                        triangles.push(tri(v_index, stack[stack_ptr-1], stack[stack_ptr]));
                        stack_ptr -= 1;
                    } else {
                        break;
                    }
                } else if convex(v_index, stack[stack_ptr], stack[stack_ptr - 1]) {
                    triangles.push(tri(v_index, stack[stack_ptr], stack[stack_ptr-1]));
                    stack_ptr -= 1;
                } else {
                    break;
                }
            }
            stack[stack_ptr + 1] = v_index;
            stack_ptr += 2;
        }
    }
    let v_index = priority[num_points-1];
    for j in 0..(stack_ptr-1) {
        if vertex_types[stack[j+1]] == 1 {
            triangles.push(tri(stack[j], stack[j+1], v_index));
        } else {
            triangles.push(tri(stack[j+1], stack[j], v_index));
        }
    }

    Ok(triangles)
}

pub fn monotone_partition(inpolys: Vec<Polygon>) -> Result<Vec<Polygon>, &'static str> {
    Ok(partition(inpolys)?
        .into_iter()
        .map(|piece| Polygon::from_points_and_is_hole(piece.iter().map(|s| s.p).collect(), false))
        .collect())
}

/// Monotone pieces, keeping each point's sweep identity so the pieces are
/// triangulated under the same perturbation they were cut with.
fn partition(inpolys: Vec<Polygon>) -> Result<Vec<Vec<SweepPoint>>, &'static str> {
    if inpolys.iter().any(|poly| !poly.is_valid()) {
        return Err("Some input polygon is invalid!");
    }

    // Repeated points and spike tips enclose nothing; a ring left with fewer
    // than three points has no area at all.
    let rings: Vec<Vec<PointF64>> = inpolys
        .iter()
        .map(|poly| remove_degenerate_points(&poly.props().points))
        .filter(|ring| ring.len() >= 3)
        .collect();
    // Touching only ever happens at shared vertices from here on.
    let rings = split_t_junctions(&rings);

    let num_vertices = rings.iter().map(|ring| ring.len()).sum::<usize>();

    let max_num_vertices = num_vertices * 3;
    let mut vertices = vec![MonotoneVertex::default(); max_num_vertices];
    let mut new_num_vertices = num_vertices;

    let mut poly_start_index = 0;
    for ring in rings.iter() {
        let num_points = ring.len();
        let poly_end_index = poly_start_index + num_points - 1;
        for i in 0..num_points {
            vertices[i + poly_start_index].p = ring[i];
            if i == 0 {
                vertices[i + poly_start_index].previous = poly_end_index;
            } else {
                vertices[i + poly_start_index].previous = i + poly_start_index - 1;
            }
            if i == num_points - 1 {
                vertices[i + poly_start_index].next = poly_start_index;
            } else {
                vertices[i + poly_start_index].next = i + poly_start_index + 1;
            }
        }
        poly_start_index = poly_end_index + 1;
    }

    resolve_shared_points(&mut vertices, num_vertices);

    let sp = |vertices: &[MonotoneVertex], i: usize| vertices[i].sweep_point(i);

    let mut priority: Vec<usize> = (0..num_vertices).collect();
    priority.sort_by(|&index1, &index2| {
        let (p1, p2) = (sp(&vertices, index1), sp(&vertices, index2));
        if sweep_before(&p1, &p2) {
            std::cmp::Ordering::Less
        } else if sweep_before(&p2, &p1) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });

    // Determine vertex types
    let mut vertex_types = vec![VertexType::Null; max_num_vertices];
    for i in 0..num_vertices {
        let v = sp(&vertices, i);
        let v_prev = sp(&vertices, vertices[i].previous);
        let v_next = sp(&vertices, vertices[i].next);

        if later(&v_prev, &v) && later(&v_next, &v) {
            if orient_sign(&v_next, &v_prev, &v) > 0 {
                vertex_types[i] = VertexType::Start;
            } else {
                vertex_types[i] = VertexType::Split;
            }
        } else if later(&v, &v_prev) && later(&v, &v_next) {
            if orient_sign(&v_next, &v_prev, &v) > 0 {
                vertex_types[i] = VertexType::End;
            } else {
                vertex_types[i] = VertexType::Merge;
            }
        } else {
            vertex_types[i] = VertexType::Regular;
        }
    }

    let mut helpers = vec![0; max_num_vertices];

    let mut edge_tree = EdgeVec::default();
    let mut edge_tree_pointers = vec![None; max_num_vertices];

    let edge_from = |vertices: &[MonotoneVertex], i: usize| {
        let next = vertices[i].next;
        ScanLineEdge {
            index: i,
            p1: vertices[i].p,
            p2: vertices[next].p,
            d1: vertices[i].d,
            d2: vertices[next].d,
        }
    };

    for &v_index in priority.iter() {
        let v = &vertices[v_index].clone();
        let v_point = sp(&vertices, v_index);
        let mut v_index2 = v_index;

        // Depending on the vertex type, do the appropriate action
        match vertex_types[v_index] {
            VertexType::Null => return Err("Vertex type is not set."),
            VertexType::Start => {
                let ptr = edge_tree.push(edge_from(&vertices, v_index));
                edge_tree_pointers[v_index] = Some(ptr);
                helpers[v_index] = v_index;
            },
            VertexType::End => {
                if edge_tree_pointers[v.previous].is_none() {
                    return Err("Pointer is NULL.");
                }

                if let VertexType::Merge = vertex_types[helpers[v.previous]] {
                    add_diagonal(&mut vertices, &mut new_num_vertices, v_index, helpers[v.previous],
                        &mut vertex_types, &mut edge_tree_pointers, &mut helpers);
                }

                if let Some(ptr) = &edge_tree_pointers[v.previous] {
                    edge_tree.remove_ptr(ptr);
                }
                edge_tree_pointers[v.previous] = None;
            },
            VertexType::Split => {
                let edge = match edge_tree.left_of(&v_point) {
                    Some(edge) => edge,
                    None => return Err("edge_iter is the first in EdgeTree."),
                };

                let helper = helpers[edge.borrow().index];
                add_diagonal(&mut vertices, &mut new_num_vertices, v_index, helper,
                    &mut vertex_types, &mut edge_tree_pointers, &mut helpers);

                v_index2 = new_num_vertices - 2;
                // add_diagonal may have handed the edge to a new copy of its
                // vertex; upstream re-reads it through the iterator too.
                helpers[edge.borrow().index] = v_index;

                let ptr = edge_tree.push(edge_from(&vertices, v_index2));
                edge_tree_pointers[v_index2] = Some(ptr);
                helpers[v_index2] = v_index2;
            },
            VertexType::Merge => {
                if edge_tree_pointers[v.previous].is_none() {
                    return Err("Pointer is NULL.");
                }

                if let VertexType::Merge = vertex_types[helpers[v.previous]] {
                    add_diagonal(&mut vertices, &mut new_num_vertices, v_index, helpers[v.previous],
                        &mut vertex_types, &mut edge_tree_pointers, &mut helpers);
                    v_index2 = new_num_vertices - 2;
                }

                if let Some(ptr) = &edge_tree_pointers[v.previous] {
                    edge_tree.remove_ptr(ptr);
                }
                edge_tree_pointers[v.previous] = None;

                let edge = match edge_tree.left_of(&v_point) {
                    Some(edge) => edge,
                    None => return Err("edge_iter is the first in EdgeTree."),
                };

                let helper = helpers[edge.borrow().index];
                if let VertexType::Merge = vertex_types[helper] {
                    add_diagonal(&mut vertices, &mut new_num_vertices, v_index2, helper,
                        &mut vertex_types, &mut edge_tree_pointers, &mut helpers);
                }

                helpers[edge.borrow().index] = v_index2;
            },
            VertexType::Regular => {
                if later(&v_point, &sp(&vertices, v.previous)) {
                    if edge_tree_pointers[v.previous].is_none() {
                        return Err("Pointer is NULL.");
                    }

                    if let VertexType::Merge = vertex_types[helpers[v.previous]] {
                        add_diagonal(&mut vertices, &mut new_num_vertices, v_index, helpers[v.previous],
                            &mut vertex_types, &mut edge_tree_pointers, &mut helpers);
                        v_index2 = new_num_vertices - 2;
                    }

                    if let Some(ptr) = &edge_tree_pointers[v.previous] {
                        edge_tree.remove_ptr(ptr);
                    }

                    let ptr = edge_tree.push(edge_from(&vertices, v_index2));
                    edge_tree_pointers[v_index2] = Some(ptr);
                    // Upstream sets this to v_index, but after a merge diagonal that
                    // copy belongs to the piece just closed off above; a later
                    // diagonal to it would sew the pieces back together.
                    helpers[v_index2] = v_index2;
                } else {
                    let edge = match edge_tree.left_of(&v_point) {
                        Some(edge) => edge,
                        None => return Err("edge_iter is the first in EdgeTree."),
                    };

                    let helper = helpers[edge.borrow().index];
                    if let VertexType::Merge = vertex_types[helper] {
                        add_diagonal(&mut vertices, &mut new_num_vertices, v_index, helper,
                            &mut vertex_types, &mut edge_tree_pointers, &mut helpers);
                    }
                    helpers[edge.borrow().index] = v_index;
                }
            },
        }
    }

    let mut monotone_polys = vec![];

    let mut used = vec![false; new_num_vertices];
    for i in 0..new_num_vertices {
        if used[i] {
            continue;
        }
        let mut piece = vec![];
        let mut v = i;
        loop {
            used[v] = true;
            piece.push(sp(&vertices, v));
            v = vertices[v].next;
            if v == i {
                break;
            }
        }
        monotone_polys.push(piece);
    }
    Ok(monotone_polys)
}
