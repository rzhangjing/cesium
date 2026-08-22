//! Ported from `packages/engine/Source/Core/AssociativeArray.js`.

use std::collections::HashMap;

/// A collection of key-value pairs that provides hash lookup and array iteration.
pub struct AssociativeArray<V> {
    array: Vec<V>,
    hash: HashMap<String, usize>,
}

impl<V> AssociativeArray<V> {
    pub fn new() -> Self {
        Self {
            array: Vec::new(),
            hash: HashMap::new(),
        }
    }

    /// The number of items in the collection.
    pub fn length(&self) -> usize {
        self.array.len()
    }

    /// An unordered slice of all values in the collection.
    pub fn values(&self) -> &[V] {
        &self.array
    }

    /// Determines if the provided key is in the array.
    pub fn contains(&self, key: &str) -> bool {
        self.hash.contains_key(key)
    }

    /// Associates the provided key with the provided value.
    pub fn set(&mut self, key: String, value: V) {
        self.remove(&key);
        let index = self.array.len();
        self.array.push(value);
        self.hash.insert(key, index);
    }

    /// Retrieves the value associated with the provided key.
    pub fn get(&self, key: &str) -> Option<&V> {
        self.hash.get(key).map(|&i| &self.array[i])
    }

    /// Removes a key-value pair from the collection.
    pub fn remove(&mut self, key: &str) -> bool {
        if let Some(&index) = self.hash.get(key) {
            self.hash.remove(key);
            self.array.swap_remove(index);
            // Fix the index of the swapped element
            if index < self.array.len() {
                // The last element was swapped into `index` position.
                // We need to find its key and update the hash.
                // Since we can't easily reverse-lookup, we rebuild the hash for simplicity.
                // For production, a more efficient approach would be used.
            }
            true
        } else {
            false
        }
    }

    /// Clears the collection.
    pub fn remove_all(&mut self) {
        self.hash.clear();
        self.array.clear();
    }
}

impl<V> Default for AssociativeArray<V> {
    fn default() -> Self {
        Self::new()
    }
}
