//! Ported from `packages/engine/Source/Core/Heap.js`.
//!
//! Array implementation of a heap.

use std::sync::Arc;

/// A heap implementation using an array.
pub struct Heap<T: Clone> {
    comparator: Arc<dyn Fn(&T, &T) -> f64 + Send + Sync>,
    array: Vec<Option<T>>,
    length: usize,
    maximum_length: Option<usize>,
}

impl<T: Clone> Heap<T> {
    /// Creates a new Heap with the given comparator.
    pub fn new(comparator: impl Fn(&T, &T) -> f64 + Send + Sync + 'static) -> Self {
        Self {
            comparator: Arc::new(comparator),
            array: Vec::new(),
            length: 0,
            maximum_length: None,
        }
    }

    /// Gets the length of the heap.
    pub fn length(&self) -> usize {
        self.length
    }

    /// Gets a reference to the internal array.
    pub fn internal_array(&self) -> &[Option<T>] {
        &self.array
    }

    /// Gets the maximum length of the heap.
    pub fn maximum_length(&self) -> Option<usize> {
        self.maximum_length
    }

    /// Sets the maximum length of the heap.
    pub fn set_maximum_length(&mut self, value: usize) {
        let original_length = self.length;
        if value < original_length {
            for i in value..original_length {
                self.array[i] = None;
            }
            self.length = value;
            self.array.truncate(value);
        }
        self.maximum_length = Some(value);
    }

    /// Resizes the internal array.
    pub fn reserve(&mut self, length: usize) {
        self.array.resize(length, None);
    }

    /// Updates the heap so that index and all descendants satisfy the heap property.
    pub fn heapify(&mut self, start_index: usize) {
        let mut index = start_index;
        let length = self.length;

        loop {
            let right = 2 * (index + 1);
            let left = right - 1;

            let mut candidate = if left < length
                && (self.comparator)(
                    self.array[left].as_ref().unwrap(),
                    self.array[index].as_ref().unwrap(),
                ) < 0.0
            {
                left
            } else {
                index
            };

            if right < length
                && (self.comparator)(
                    self.array[right].as_ref().unwrap(),
                    self.array[candidate].as_ref().unwrap(),
                ) < 0.0
            {
                candidate = right;
            }

            if candidate != index {
                self.array.swap(index, candidate);
                index = candidate;
            } else {
                break;
            }
        }
    }

    /// Resorts the heap.
    pub fn resort(&mut self) {
        let length = self.length;
        let mut i = (length as f64 / 2.0).ceil() as isize;
        while i >= 0 {
            self.heapify(i as usize);
            i -= 1;
        }
    }

    /// Inserts an element into the heap.
    pub fn insert(&mut self, element: T) -> Option<T> {
        let index = self.length;
        self.length += 1;

        if index < self.array.len() {
            self.array[index] = Some(element);
        } else {
            self.array.push(Some(element));
        }

        let mut current = index;
        while current != 0 {
            let parent = (current as isize - 1) / 2;
            let parent_idx = parent as usize;
            if (self.comparator)(
                self.array[current].as_ref().unwrap(),
                self.array[parent_idx].as_ref().unwrap(),
            ) < 0.0
            {
                self.array.swap(current, parent_idx);
                current = parent_idx;
            } else {
                break;
            }
        }

        let mut removed_element = None;
        if let Some(max_len) = self.maximum_length {
            if self.length > max_len {
                removed_element = self.array[max_len].take();
                self.length = max_len;
            }
        }

        removed_element
    }

    /// Removes and returns the element at the given index (default 0).
    pub fn pop(&mut self, index: usize) -> Option<T> {
        if self.length == 0 {
            return None;
        }

        self.length -= 1;
        self.array.swap(index, self.length);
        self.heapify(index);
        let result = self.array[self.length].take();
        result
    }

    /// Removes and returns the root element.
    pub fn pop_root(&mut self) -> Option<T> {
        self.pop(0)
    }
}
