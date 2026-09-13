//! A managed array that tracks length separately from capacity.
//! Maps to CesiumJS `Core/ManagedArray.js`

/// An array-like data structure that manages its own capacity,
/// tracking logical length separately from reserved capacity.
#[derive(Debug, Clone)]
pub struct ManagedArray<T: Default + Clone> {
    values: Vec<T>,
    length: usize,
}

impl<T: Default + Clone> ManagedArray<T> {
    /// Creates a new ManagedArray with the given initial length.
    /// The internal storage is initialized to `length` elements.
    pub fn new(length: usize) -> Self {
        Self {
            values: vec![T::default(); length],
            length,
        }
    }

    /// Returns the logical length of the array.
    pub fn length(&self) -> usize {
        self.length
    }

    /// Sets the logical length. If growing, new elements are default-initialized.
    /// If shrinking, the capacity is preserved.
    pub fn set_length(&mut self, length: usize) {
        self.resize(length);
    }

    /// Returns a reference to the internal values slice (up to reserved capacity).
    pub fn values(&self) -> &[T] {
        &self.values
    }

    /// Returns the reserved capacity (internal storage length).
    pub fn capacity(&self) -> usize {
        self.values.len()
    }

    /// Gets the element at the given index.
    ///
    /// # Panics
    /// Panics if `index >= length`.
    pub fn get(&self, index: usize) -> &T {
        assert!(index < self.length, "index out of bounds");
        &self.values[index]
    }

    /// Sets the element at the given index, resizing if necessary.
    pub fn set(&mut self, index: usize, value: T) {
        if index >= self.length {
            self.resize(index + 1);
        }
        self.values[index] = value;
    }

    /// Returns the last element, or None if empty.
    pub fn peek(&self) -> Option<&T> {
        if self.length == 0 {
            None
        } else {
            Some(&self.values[self.length - 1])
        }
    }

    /// Pushes a value onto the end of the array.
    pub fn push(&mut self, value: T) {
        if self.length < self.values.len() {
            self.values[self.length] = value;
        } else {
            self.values.push(value);
        }
        self.length += 1;
    }

    /// Pops the last element from the array.
    /// Returns None if the array is empty.
    pub fn pop(&mut self) -> Option<T> {
        if self.length == 0 {
            return None;
        }
        self.length -= 1;
        let value = self.values[self.length].clone();
        self.values[self.length] = T::default();
        Some(value)
    }

    /// Reserves at least `capacity` elements of internal storage.
    /// Does not change the logical length.
    pub fn reserve(&mut self, capacity: usize) {
        if capacity > self.values.len() {
            self.values.resize(capacity, T::default());
        }
    }

    /// Resizes the logical length. If growing, new elements are default-initialized.
    /// If shrinking, capacity is preserved but trailing elements are cleared.
    pub fn resize(&mut self, length: usize) {
        if length > self.values.len() {
            self.values.resize(length, T::default());
        }
        // Clear trailing references when shrinking
        if length < self.length {
            for i in length..self.length {
                if i < self.values.len() {
                    self.values[i] = T::default();
                }
            }
        }
        self.length = length;
    }

    /// Trims the internal storage to the given capacity (or current length if not specified).
    pub fn trim(&mut self, capacity: Option<usize>) {
        let target = capacity.unwrap_or(self.length).max(self.length);
        self.values.resize(target, T::default());
    }
}
