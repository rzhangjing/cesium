//! Maps to CesiumJS `Core/AssociativeArray.js`
//!
//! A collection of key-value pairs that is stored as a hash for easy lookup
//! but also provides an array for fast iteration.

use std::collections::HashMap;

/// A collection of key-value pairs stored as a hash for easy lookup while
/// also maintaining an array of values for fast iteration.
pub struct AssociativeArray<T> {
    array: Vec<T>,
    hash: HashMap<String, T>,
}

impl<T> Default for AssociativeArray<T>
where
    T: Clone + PartialEq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> AssociativeArray<T>
where
    T: Clone + PartialEq,
{
    /// Creates a new, empty associative array.
    pub fn new() -> Self {
        Self {
            array: Vec::new(),
            hash: HashMap::new(),
        }
    }

    /// Gets the number of items in the collection.
    pub fn length(&self) -> usize {
        self.array.len()
    }

    /// Gets the array of all values in the collection.
    pub fn values(&self) -> &[T] {
        &self.array
    }

    /// Determines if the provided key is in the array.
    pub fn contains(&self, key: &str) -> bool {
        self.hash.contains_key(key)
    }

    /// Associates the provided key with the provided value. If the key already
    /// exists, it is overwritten with the new value.
    pub fn set(&mut self, key: &str, value: T) {
        let needs_update = match self.hash.get(key) {
            Some(old) => *old != value,
            None => true,
        };
        if needs_update {
            self.remove(key);
            self.array.push(value.clone());
            self.hash.insert(key.to_string(), value);
        }
    }

    /// Retrieves the value associated with the provided key, or `None` if the
    /// key does not exist in the collection.
    pub fn get(&self, key: &str) -> Option<&T> {
        self.hash.get(key)
    }

    /// Removes a key-value pair from the collection.
    /// Returns `true` if it was removed, `false` if the key was not present.
    pub fn remove(&mut self, key: &str) -> bool {
        if let Some(value) = self.hash.remove(key) {
            if let Some(idx) = self.array.iter().position(|v| *v == value) {
                self.array.remove(idx);
            }
            true
        } else {
            false
        }
    }

    /// Clears the collection.
    pub fn remove_all(&mut self) {
        if !self.array.is_empty() {
            self.hash.clear();
            self.array.clear();
        }
    }
}
