//! Contains functions and data structures for partitioning items into groups.
//!
//! The symbols in this module is part of visioncortex's public API, but are generally
//! only useful for internal implementations.
use std::{hash::Hash, collections::HashMap};

/// Groups items with a key extraction function and a equivalence testing function on the keys.
/// See the documentation of `group_by` for the requirements of the testing function.
///
/// During grouping, the key function is called only once per element.
///
/// For simple key functions, `group_by` is likely to be faster.
///
/// # Example
/// ```
/// use visioncortex::disjoint_sets::group_by_cached_key;
/// let points = vec![1,1,7,9,24,1,4,7,3,8];
/// let groups = group_by_cached_key(points, |&x| x, |&x, &y| {
///     (x - y) * (x - y) < 2
/// });
/// // should be grouped as below:
/// // {1, 1, 1}, {3, 4}, {7, 7, 8, 9}, {24}
/// for mut group in groups {
///     println!("{:?}", group);
///     group.sort();
///     if group.len() == 4 {
///         assert_eq!(group, [7, 7, 8, 9]);
///     } else if group.len() == 3 {
///         assert_eq!(group, [1, 1, 1]);
///     } else if group.len() == 2 {
///         assert_eq!(group, [3, 4]);
///     } else {
///         assert_eq!(group, [24]);
///     }
/// }
/// ```
pub fn group_by_cached_key<T, Key, Extract, Group> (
    items: Vec<T>,
    extract_key: Extract,
    should_group: Group
) -> Vec<Vec<T>>
where
    Extract: Fn(&T) -> Key,
    Group: Fn(&Key, &Key) -> bool,
{
    let items_with_keys = items
        .into_iter()
        .map(|item| {
            let k = extract_key(&item);
            (item, k)
        })
        .collect();

    group_by(items_with_keys, |(_, key1), (_, key2)| should_group(key1, key2))
        .into_iter()
        .map(|group| group.into_iter().map(|(item, _)| item).collect())
        .collect()
}

/// Groups items with a equivalence testing function.
///
/// The testing function should define a equivalence relation `~` on the set of elements
/// and return true for elements `a` and `b` if-and-only-if `a ~ b`.
/// This implies that the function is commutative, i.e. `should_group(a, b) == should_group(b, a`).
///
/// # Example
/// ```
/// use visioncortex::disjoint_sets::group_by;
/// let points = vec![1,1,7,9,24,1,4,7,3,8];
/// let groups = group_by(points, |&x, &y| {
///     (x - y) * (x - y) < 2
/// });
/// // should be grouped as below:
/// // {1, 1, 1}, {3, 4}, {7, 7, 8, 9}, {24}
/// for mut group in groups {
///     println!("{:?}", group);
///     group.sort();
///     if group.len() == 4 {
///         assert_eq!(group, [7, 7, 8, 9]);
///     } else if group.len() == 3 {
///         assert_eq!(group, [1, 1, 1]);
///     } else if group.len() == 2 {
///         assert_eq!(group, [3, 4]);
///     } else {
///         assert_eq!(group, [24]);
///     }
/// }
/// ```
pub fn group_by<T, F>(mut items: Vec<T>, should_group: F) -> Vec<Vec<T>> 
where
    F: Fn(&T, &T) -> bool,
{
    let mut forests = Forests::new();
    for i in 0..items.len() {
        forests.make_set(i);
    }

    for (i, item1) in items.iter().enumerate() {
        for (j, item2) in items.iter().enumerate().skip(i + 1) {
            if should_group(item1, item2) {
                forests.union(&i, &j);
            }
        }
    }

    let mut group_index = HashMap::new();
    let mut groups = Vec::new();
    
    while let Some(item) = items.pop() {
        let index = items.len();
        let label = forests.find_set(&index).unwrap(); // safe because we already made sets 0..n
        
        if let Some(&i) = group_index.get(&label) {
            let group: &mut Vec<T> = &mut groups[i]; // to bypass 'type annotation needed'
            group.push(item);
        } else {
            group_index.insert(label, groups.len());
            groups.push(vec![item]);
        }
    }

    groups
}

pub type Label = u32;

/// Data structure for building disjoint sets
pub struct Forests<T>
where
    T: Eq + Hash,
{
    parents: Vec<Label>,
    ranks: Vec<u8>,
    labels: HashMap<T, Label>,
}

impl<T> Default for Forests<T>
where
    T: Eq + Hash,
{
    fn default() -> Self {
        Self {
            parents: vec![],
            ranks: vec![],
            labels: HashMap::new(),
        }
    }
}

impl<T> Forests<T>
where
    T: Eq + Hash,
{
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Counts the number of unique disjoint sets.
    pub fn count_sets(&mut self) -> usize {
        use std::collections::HashSet;
        let mut roots = HashSet::new();
        
        for i in 0..self.parents.len() as u32 {
            let root = self.find_and_compress_path(i);
            roots.insert(root);
        }

        roots.len()
    }

    /// Groups `items` by their containing sets. The result is the indices of items in the provided `items`
    /// that belongs to different disjoint sets. The order of groups is arbitrary.
    /// Items that do not exist in the forest belongs to the same group that does not consist of other contained items.
    pub fn group_items(&mut self, items: &[T]) -> Vec<Vec<usize>> {
        let mut groups = HashMap::new();
        let mut not_exists = vec![];

        for (i, item) in items.iter().enumerate() {
            if let Some(root) = self.find_set(item) {
                let group = groups.entry(root).or_insert_with(Vec::new);
                group.push(i);
            } else {
                not_exists.push(i);
            }
        }

        let mut groups: Vec<_> = groups.into_iter().map(|(_, v)| v).collect();
        if !not_exists.is_empty() {
            groups.push(not_exists);
        }
        groups
    }

    /// Makes a new singleton set with exactly one element `item`.
    pub fn make_set(&mut self, item: T) {
        if self.labels.contains_key(&item) {
            return;
        }

        // The new label of `item` should be the next available index.
        let label = self.ranks.len() as Label;
        self.labels.insert(item, label);
        self.parents.push(label); // parent points to item itself
        self.ranks.push(0);
    }

    /// Find the label of the set `item` belongs to.
    pub fn find_set(&mut self, item: &T) -> Option<Label> {
        self.labels.get(item).copied().map(|label| self.find_and_compress_path(label))
    }

    /// Finds the root label of `label`, compressing the path along the traversal towards root as a side effect.
    fn find_and_compress_path(&mut self, label: Label) -> Label {
        let mut path_visited = vec![];
        let mut cur = label;

        loop {
            // traverse towards parent until parent == itself
            let parent = self.parents[cur as usize];
            if parent == cur {
                break;
            }
            path_visited.push(cur);
            cur = parent;
        }

        // compress path
        for visited in path_visited {
            self.parents[visited as usize] = cur;
        }

        cur
    }

    /// Unions the two sets containing `item1` and `item2`.
    /// No-op if either `item1` or `item2` is not present (i.e. no `make_set` has been made).
    pub fn union(&mut self, item1: &T, item2: &T) {
        if let (Some(root1), Some(root2)) = (self.find_set(item1), self.find_set(item2)) {
            self.link(root1, root2);
        }
    }

    /// Implements union by rank.
    fn link(&mut self, x: Label, y: Label) {
        match self.ranks[x as usize].cmp(&self.ranks[y as usize]) {
            std::cmp::Ordering::Greater => self.parents[y as usize] = x,
            std::cmp::Ordering::Less => self.parents[x as usize] = y,
            std::cmp::Ordering::Equal => {
                // break ties arbitrarily
                self.parents[x as usize] = y;
                self.ranks[y as usize] += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_find() {
        let mut forests = Forests::new();
        for i in 1..11 {
            forests.make_set(i);
        }
        forests.union(&2, &4);
        forests.union(&5, &7);
        forests.union(&1, &3);
        forests.union(&8, &9);
        forests.union(&1, &2);
        forests.union(&5, &6);
        forests.union(&2, &3);

        assert_eq!(forests.find_set(&1), forests.find_set(&2));
        assert_eq!(forests.find_set(&2), forests.find_set(&3));
        assert_eq!(forests.find_set(&3), forests.find_set(&4));

        assert_eq!(forests.find_set(&5), forests.find_set(&6));
        assert_eq!(forests.find_set(&6), forests.find_set(&7));

        assert_eq!(forests.find_set(&8), forests.find_set(&9));

        assert_ne!(forests.find_set(&10), forests.find_set(&1));
        assert_ne!(forests.find_set(&1), forests.find_set(&5));
        assert_ne!(forests.find_set(&6), forests.find_set(&8));

        assert_eq!(forests.count_sets(), 4);

        let items: Vec<_> = (1..11).collect();
        let groups = forests.group_items(&items);

        for group in groups {
            if group.len() == 4 {
                assert_eq!(group, [0, 1, 2, 3]);
            } else if group.len() == 3 {
                assert_eq!(group, [4, 5, 6]);
            } else if group.len() == 2 {
                assert_eq!(group, [7, 8]);
            } else {
                assert_eq!(group, [9]);
            }
        }
    }

    #[test]
    fn group_items() {
        let points = vec![1,1,7,9,24,1,4,7,3,8];
        let groups = group_by(points, |&x, &y| {
            (x - y) * (x - y) < 2
        });
        // should be grouped as below:
        // {1, 1, 1}, {3, 4}, {7, 7, 8, 9}, {24}
        for mut group in groups {
            println!("{:?}", group);
            group.sort();
            if group.len() == 4 {
                assert_eq!(group, [7, 7, 8, 9]);
            } else if group.len() == 3 {
                assert_eq!(group, [1, 1, 1]);
            } else if group.len() == 2 {
                assert_eq!(group, [3, 4]);
            } else {
                assert_eq!(group, [24]);
            }
        }
    }

    #[test]
    fn group_cached() {
        let points = vec![1,1,7,9,24,1,4,7,3,8];
        let groups = group_by_cached_key(points, |&x| x, |&x, &y| {
            (x - y) * (x - y) < 2
        });
        // should be grouped as below:
        // {1, 1, 1}, {3, 4}, {7, 7, 8, 9}, {24}
        for mut group in groups {
            println!("{:?}", group);
            group.sort();
            if group.len() == 4 {
                assert_eq!(group, [7, 7, 8, 9]);
            } else if group.len() == 3 {
                assert_eq!(group, [1, 1, 1]);
            } else if group.len() == 2 {
                assert_eq!(group, [3, 4]);
            } else {
                assert_eq!(group, [24]);
            }
        }
    }
}