//! A 2D spatial index for collision detection.
//!
//! The canvas is divided into roughly-square tiles. Items are inserted into
//! the tile that contains their position. Queries return only the items in the
//! relevant tile(s), reducing collision checks from O(n²) to O(items per tile).

use std::collections::HashSet;
use crate::{BoundingRect, PointI32};

/// A 2D spatial index that buckets items by tile position.
///
/// The canvas is divided into a grid of tiles. Each tile holds a `Vec<T>` of
/// items whose insertion point falls within that tile's area.
///
/// # Layout
/// Tiles are stored in a flat `Vec` indexed by `tile_y * cols + tile_x`.
///
/// # Default tile size
/// `max(canvas_width, canvas_height) / 10`, giving roughly 100 tiles for a
/// standard canvas. Use [`TileMap::with_tile_size`] to override.
pub struct TileMap<T> {
    tiles: Vec<Vec<T>>,  // tiles[tile_y * cols + tile_x] = bucket of items in that tile
    tile_size: i32,
    cols: i32,           // number of tile columns
    rows: i32,           // number of tile rows
}

impl<T> TileMap<T> {
    /// Creates a new `TileMap` for a canvas of the given pixel dimensions.
    ///
    /// Tile size is set to `max(width, height) / 10`.
    pub fn new(width: i32, height: i32) -> Self {
        let tile_size = width.max(height) / 10;
        Self::with_tile_size(width, height, tile_size)
    }

    /// Creates a new `TileMap` with an explicit tile size.
    pub fn with_tile_size(width: i32, height: i32, tile_size: i32) -> Self {
        assert!(tile_size > 0, "tile_size must be positive");
        let cols = (width  + tile_size - 1) / tile_size;
        let rows = (height + tile_size - 1) / tile_size;
        let count = (cols * rows) as usize;
        Self {
            tiles: (0..count).map(|_| Vec::new()).collect(),
            tile_size,
            cols,
            rows,
        }
    }

    /// Inserts `value` into the tile that contains point `p`.
    ///
    /// Points outside the canvas bounds are clamped to the nearest tile.
    pub fn add_point(&mut self, p: PointI32, value: T) {
        let (tx, ty) = self.tile_coords(p);
        let idx = self.tile_index(tx, ty);
        self.tiles[idx].push(value);
    }

    /// Inserts `value` into every perimeter tile touched by `rect`'s edges.
    ///
    /// Uses the same tile-walking logic as [`query_rect`]: only the 4 edge
    /// strips are visited, interior tiles are skipped, and duplicates are
    /// suppressed so `value` is inserted at most once per tile.
    ///
    /// Requires `T: Copy` because the same value may be written into multiple tiles.
    pub fn add_rect(&mut self, rect: &BoundingRect, value: T) where T: Copy {
        let tx_min = (rect.left   / self.tile_size).clamp(0, self.cols - 1);
        let tx_max = (rect.right  / self.tile_size).clamp(0, self.cols - 1);
        let ty_min = (rect.top    / self.tile_size).clamp(0, self.rows - 1);
        let ty_max = (rect.bottom / self.tile_size).clamp(0, self.rows - 1);

        let mut seen: HashSet<usize> = HashSet::new();

        for tx in tx_min..=tx_max {
            seen.insert(self.tile_index(tx, ty_min));
            seen.insert(self.tile_index(tx, ty_max));
        }
        for ty in ty_min..=ty_max {
            seen.insert(self.tile_index(tx_min, ty));
            seen.insert(self.tile_index(tx_max, ty));
        }

        for idx in seen {
            self.tiles[idx].push(value);
        }
    }

    /// Returns all items in the tile that contains point `p`.
    ///
    /// Points outside the canvas bounds are clamped to the nearest tile.
    pub fn query_point(&self, p: &PointI32) -> &[T] {
        let (tx, ty) = self.tile_coords(*p);
        let idx = self.tile_index(tx, ty);
        &self.tiles[idx]
    }

    /// Returns the items from every perimeter tile touched by `rect`'s edges.
    ///
    /// Only the tiles along the **4 edges** of the rect are visited — interior
    /// tiles are skipped. This makes `query_rect` a natural companion to
    /// [`BoundingRect::intersect`], which tests for edge crossings only (not
    /// containment). Empty tiles are omitted from the result.
    ///
    /// Each returned `&[T]` slice corresponds to one distinct tile. Tiles are
    /// deduplicated by tile id before returning, so no tile appears twice even
    /// when the rect fits within a single tile.
    pub fn query_rect(&self, rect: &BoundingRect) -> Vec<&[T]> {
        let tx_min = (rect.left   / self.tile_size).clamp(0, self.cols - 1);
        let tx_max = (rect.right  / self.tile_size).clamp(0, self.cols - 1);
        let ty_min = (rect.top    / self.tile_size).clamp(0, self.rows - 1);
        let ty_max = (rect.bottom / self.tile_size).clamp(0, self.rows - 1);

        let mut seen: HashSet<usize> = HashSet::new();

        // top and bottom edges (full horizontal span)
        for tx in tx_min..=tx_max {
            seen.insert(self.tile_index(tx, ty_min));
            seen.insert(self.tile_index(tx, ty_max));
        }
        // left and right edges (full vertical span — duplicates removed by HashSet)
        for ty in ty_min..=ty_max {
            seen.insert(self.tile_index(tx_min, ty));
            seen.insert(self.tile_index(tx_max, ty));
        }

        seen.into_iter()
            .filter_map(|idx| {
                let bucket = &self.tiles[idx];
                if bucket.is_empty() { None } else { Some(bucket.as_slice()) }
            })
            .collect()
    }

    // --- private helpers ---

    fn tile_coords(&self, p: PointI32) -> (i32, i32) {
        let tx = (p.x / self.tile_size).clamp(0, self.cols - 1);
        let ty = (p.y / self.tile_size).clamp(0, self.rows - 1);
        (tx, ty)
    }

    fn tile_index(&self, tx: i32, ty: i32) -> usize {
        (ty * self.cols + tx) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> BoundingRect {
        BoundingRect { left, top, right, bottom }
    }

    #[test]
    fn new_computes_grid_dimensions() {
        // 1920x1080 → tile_size=192, cols=10, rows=6 (ceil(1080/192)=6)
        let map = TileMap::<u32>::new(1920, 1080);
        assert_eq!(map.tile_size, 192);
        assert_eq!(map.cols, 10);
        assert_eq!(map.rows, 6);
        assert_eq!(map.tiles.len(), 60);
    }

    #[test]
    fn with_tile_size_override() {
        let map = TileMap::<u32>::with_tile_size(100, 100, 10);
        assert_eq!(map.tile_size, 10);
        assert_eq!(map.cols, 10);
        assert_eq!(map.rows, 10);
    }

    #[test]
    fn add_point_and_query_point_roundtrip() {
        let mut map = TileMap::with_tile_size(100, 100, 10);
        map.add_point(PointI32::new(5, 5), 42u32);
        assert_eq!(map.query_point(&PointI32::new(5, 5)), &[42]);
    }

    #[test]
    fn query_point_same_tile_returns_all() {
        let mut map = TileMap::with_tile_size(100, 100, 10);
        map.add_point(PointI32::new(0, 0), 1u32);
        map.add_point(PointI32::new(9, 9), 2u32); // same tile as (0,0)
        map.add_point(PointI32::new(10, 0), 3u32); // different tile
        let result = map.query_point(&PointI32::new(0, 0));
        assert_eq!(result, &[1, 2]);
    }

    #[test]
    fn query_point_empty_tile() {
        let map = TileMap::<u32>::with_tile_size(100, 100, 10);
        assert!(map.query_point(&PointI32::new(50, 50)).is_empty());
    }

    #[test]
    fn point_on_tile_boundary_goes_to_lower_tile() {
        let mut map = TileMap::with_tile_size(100, 100, 10);
        // x=10 → tile 1, not tile 0
        map.add_point(PointI32::new(10, 0), 99u32);
        assert!(map.query_point(&PointI32::new(0, 0)).is_empty());
        assert_eq!(map.query_point(&PointI32::new(10, 0)), &[99]);
    }

    #[test]
    fn out_of_bounds_point_clamped() {
        let mut map = TileMap::with_tile_size(100, 100, 10);
        map.add_point(PointI32::new(200, 200), 7u32); // clamped to last tile
        assert_eq!(map.query_point(&PointI32::new(99, 99)), &[7]);
    }

    #[test]
    fn query_rect_single_tile() {
        let mut map = TileMap::with_tile_size(100, 100, 10);
        map.add_point(PointI32::new(5, 5), 1u32);
        // rect entirely within tile (0,0)
        let result = map.query_rect(&rect(0, 0, 9, 9));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], &[1]);
    }

    #[test]
    fn query_rect_spans_multiple_tiles() {
        let mut map = TileMap::with_tile_size(100, 100, 10);
        map.add_point(PointI32::new(5,  5),  1u32); // tile (0,0)
        map.add_point(PointI32::new(15, 5),  2u32); // tile (1,0)
        map.add_point(PointI32::new(25, 5),  3u32); // tile (2,0)
        map.add_point(PointI32::new(50, 50), 9u32); // interior tile — should NOT appear
        // rect spanning tiles (0,0)..(2,0) horizontally, single row
        let result = map.query_rect(&rect(0, 0, 29, 9));
        let mut flat: Vec<u32> = result.iter().flat_map(|s| s.iter().copied()).collect();
        flat.sort();
        assert_eq!(flat, vec![1, 2, 3]);
    }

    #[test]
    fn query_rect_skips_interior_tiles() {
        let mut map = TileMap::with_tile_size(100, 100, 10);
        map.add_point(PointI32::new(55, 55), 42u32); // interior tile (5,5)
        // large rect whose perimeter does not include tile (5,5)
        let result = map.query_rect(&rect(0, 0, 99, 99));
        let flat: Vec<u32> = result.iter().flat_map(|s| s.iter().copied()).collect();
        assert!(!flat.contains(&42), "interior tile should not appear in query_rect");
    }

    #[test]
    fn add_rect_inserts_into_perimeter_tiles() {
        let mut map = TileMap::with_tile_size(100, 100, 10);
        // rect spanning tiles (0,0)..(2,2) — 3x3 tile grid
        // perimeter: top row ty=0, bottom row ty=2, left col tx=0, right col tx=2
        // interior tile (1,1) must NOT receive the value
        map.add_rect(&rect(0, 0, 29, 29), 7u32);

        assert!(!map.query_point(&PointI32::new(15, 15)).is_empty() == false,
            "interior tile (1,1) should be empty");
        assert_eq!(map.query_point(&PointI32::new(5,  5)),  &[7]); // corner (0,0)
        assert_eq!(map.query_point(&PointI32::new(25, 5)),  &[7]); // corner (2,0)
        assert_eq!(map.query_point(&PointI32::new(5,  25)), &[7]); // corner (0,2)
        assert_eq!(map.query_point(&PointI32::new(25, 25)), &[7]); // corner (2,2)
        assert_eq!(map.query_point(&PointI32::new(15, 5)),  &[7]); // top edge (1,0)
        assert_eq!(map.query_point(&PointI32::new(5,  15)), &[7]); // left edge (0,1)
    }

    #[test]
    fn add_rect_single_tile_no_duplicate() {
        let mut map = TileMap::with_tile_size(100, 100, 10);
        // rect fits within one tile — value inserted exactly once
        map.add_rect(&rect(0, 0, 5, 5), 3u32);
        assert_eq!(map.query_point(&PointI32::new(0, 0)), &[3]);
    }

    #[test]
    fn add_rect_skips_interior() {
        let mut map = TileMap::with_tile_size(100, 100, 10);
        map.add_rect(&rect(0, 0, 99, 99), 1u32);
        // interior tile (5,5) should be empty
        assert!(map.query_point(&PointI32::new(55, 55)).is_empty());
    }

    #[test]
    fn intersecting_large_rects_no_shared_corner_tile() {
        // tile_size = 10
        //
        // Rect A: (0,0)..(49,49)  → tile corners at (0,0),(4,0),(0,4),(4,4)
        // Rect B: (15,15)..(59,59) → tile corners at (1,1),(5,1),(1,5),(5,5)
        //
        // No corner tiles are shared between A and B.
        // However A's right edge (tx=4) and B's top edge (ty=1) cross at tile (4,1),
        // so query_rect(B) must find the value registered for A.
        // Rect C: (70,70)..(89,89) → tile corners at (7,7),(8,7),(7,8),(8,8)
        // Its perimeter tiles are entirely outside B's perimeter — should not appear.
        let mut map = TileMap::with_tile_size(100, 100, 10);
        map.add_rect(&rect(0,  0,  49, 49), 1u32); // rect A — intersects B
        map.add_rect(&rect(70, 70, 89, 89), 2u32); // rect C — disjoint from B

        let results = map.query_rect(&rect(15, 15, 59, 59)); // query rect B
        let flat: Vec<u32> = results.iter().flat_map(|s| s.iter().copied()).collect();

        assert!(flat.contains(&1), "rect A should be found via shared perimeter tile (4,1)");
        assert!(!flat.contains(&2), "rect C is disjoint from B and must not appear");
    }

    #[test]
    fn query_rect_no_duplicates() {
        let mut map = TileMap::with_tile_size(100, 100, 10);
        map.add_point(PointI32::new(0, 0), 1u32);
        // tiny rect within a single tile — all 4 edges map to the same tile
        let result = map.query_rect(&rect(0, 0, 5, 5));
        assert_eq!(result.len(), 1);
    }
}
