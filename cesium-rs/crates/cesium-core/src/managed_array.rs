//! Ported from `packages/engine/Source/Core/ManagedArray.js`.
//!
//! A wrapper around arrays so that the internal length can be manually managed.

/// An array wrapper with manual length management.
pub struct ManagedArray<T: Default + Clone> {
    array: Vec<T>,
    length: usize,
}

impl<T: Default + Clone> ManagedArray<T> {
    /// Creates a new ManagedArray with the given initial length.
    pub fn new(length: usize) -> Self {
        Self {
            array: vec![T::default(); length],
            length,
        }
    }

    /// Gets the logical length.
    pub fn length(&self) -> usize {
        self.length
    }

    /// Sets the logical length.
    pub fn set_length(&mut self, new_length: usize) {
        let original_length = self.length;
        if new_length > self.array.len() {
            self.array.resize(new_length, T::default());
        }
        self.length = new_length;
        let _ = original_length;
    }

    /// Gets a reference to the internal values.
    pub fn values(&self) -> &[T] {
        &self.array[..self.length]
    }

    /// Gets a mutable reference to the internal values.
    pub fn values_mut(&mut self) -> &mut [T] {
        &mut self.array[..self.length]
    }

    /// Gets the element at an index.
    pub fn get(&self, index: usize) -> &T {
        &self.array[index]
    }

    /// Sets the element at an index. Resizes if needed.
    pub fn set(&mut self, index: usize, element: T) {
        if index >= self.length {
            self.set_length(index + 1);
        }
        if index >= self.array.len() {
            self.array.resize(index + 1, T::default());
        }
        self.array[index] = element;
    }

    /// Returns the last element without removing it.
    pub fn peek(&self) -> Option<&T> {
        if self.length == 0 {
            return None;
        }
        Some(&self.array[self.length - 1])
    }

    /// Pushes an element.
    pub fn push(&mut self, element: T) {
        let index = self.length;
        self.length += 1;
        if index < self.array.len() {
            self.array[index] = element;
        } else {
            self.array.push(element);
        }
    }

    /// Pops the last element.
    pub fn pop(&mut self) -> Option<T> {
        if self.length == 0 {
            return None;
        }
        self.length -= 1;
        let element = std::mem::take(&mut self.array[self.length]);
        Some(element)
    }

    /// Reserves capacity.
    pub fn reserve(&mut self, length: usize) {
        if length > self.array.len() {
            self.array.resize(length, T::default());
        }
    }

    /// Resizes the logical length.
    pub fn resize(&mut self, length: usize) {
        self.set_length(length);
    }

    /// Trims the internal array to the specified length.
    pub fn trim(&mut self, length: Option<usize>) {
        let target = length.unwrap_or(self.length);
        self.array.truncate(target);
    }
}
