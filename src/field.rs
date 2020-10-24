/// `Vec<T>` that represents a 2D field
#[derive(Clone, Default)]
pub struct Field<T> {
    data: Vec<T>,
    width: usize,
    height: usize,
}

/// The base implementation of `Field`
impl<T> Field<T> {
    /// Constructs a `height` by `width` field with `data`.
    /// Returns `None` if the size of `data` does not equal to `height * width`.
    pub fn with_vec(width: usize, height: usize, data: Vec<T>) -> Option<Self> {
        if data.len() == width * height {
            Some(Self {
                data,
                width,
                height,
            })
        } else {
            None
        }
    }

    /// Returns the width of the field.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the height of the field.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns a reference to an element.
    pub fn peek(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }

    /// Returns a mutable reference to an element.
    pub fn peek_mut(&mut self, index: usize) -> Option<&mut T> {
        self.data.get_mut(index)
    }

    /// Replaces an element with `value` and returns the element.
    pub fn replace(&mut self, index: usize, value: T) -> Option<T> {
        self.data
            .get_mut(index)
            .map(|elem| std::mem::replace(elem, value))
    }

    /// Returns the xy-coordinates representation of an index in the field.
    pub fn locate(&self, index: usize) -> (usize, usize) {
        (index % self.width, index / self.width)
    }

    /// Returns the index of a xy-coordinates representation in the field.
    pub fn index_at(&self, x: usize, y: usize) -> usize {
        self.width * y + x
    }

    pub fn iter(&self) -> impl Iterator + '_ {
        self.data.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator + '_ {
        self.data.iter_mut()
    }
}

impl<T> Field<T>
where
    T: Default,
{
    /// Constructs a `height` by `width` field, with each slot filled with the default value of its element type.
    pub fn with_default(width: usize, height: usize) -> Self {
        let size = width * height;
        let mut data = Vec::with_capacity(size);
        data.resize_with(size, T::default);
        Self {
            data,
            width,
            height,
        }
    }
}

impl<T> Field<T>
where
    T: Clone,
{
    /// Constructs a `height` by `width` field, with each slot filled with `value`.
    pub fn with_initial(width: usize, height: usize, value: T) -> Self {
        Self {
            data: vec![value; width * height],
            width,
            height,
        }
    }

    /// Returns a copy of an element.
    pub fn get(&self, index: usize) -> Option<T> {
        self.peek(index).cloned()
    }

    /// Assigns a copy of a value to a slot.
    pub fn set(&mut self, index: usize, value: &T) {
        if let Some(elem) = self.data.get_mut(index) {
            *elem = value.clone();
        }
    }
}

impl<T> IntoIterator for Field<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let mut field = Field::<i32>::default();
        assert_eq!(field.width(), 0);
        assert_eq!(field.height(), 0);
        assert_eq!(field.peek(0), None);
        assert_eq!(field.peek_mut(0), None);
    }

    #[test]
    fn two_by_three() {
        let mut field = Field::with_initial(3, 2, 1);

        for i in 0..6 {
            assert_eq!(field.replace(i, i), Some(1));
        }

        for i in 0..6 {
            assert_eq!(field.peek(i), Some(&i));
        }
    }
}
