//! Ported from `packages/engine/Source/Core/AssociativeArray.js`.

use std::collections::HashMap;

/// A collection of key-value pairs that provides hash lookup and array iteration.
pub struct AssociativeArray<V> {
    array: Vec<V>,
    /// Parallel to `array`: the key of each stored value, enabling O(1)
    /// reverse lookup when `swap_remove` moves the last element.
    keys: Vec<String>,
    hash: HashMap<String, usize>,
}

impl<V> AssociativeArray<V> {
    pub fn new() -> Self {
        Self {
            array: Vec::new(),
            keys: Vec::new(),
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
    ///
    /// Mirrors the JS `set` early-out `if (value !== oldValue)`: setting the
    /// identical value for an existing key is a no-op.
    ///
    /// DEVIATION: JS compares with `!==` (reference identity for objects);
    /// the Rust port compares with `PartialEq`, which matches JS number and
    /// string semantics but is structural for object-like payloads.
    pub fn set(&mut self, key: String, value: V)
    where
        V: PartialEq,
    {
        if let Some(&index) = self.hash.get(&key) {
            if self.array[index] == value {
                return;
            }
        }
        self.remove(&key);
        let index = self.array.len();
        self.array.push(value);
        self.keys.push(key.clone());
        self.hash.insert(key, index);
    }

    /// Retrieves the value associated with the provided key.
    pub fn get(&self, key: &str) -> Option<&V> {
        self.hash.get(key).map(|&i| &self.array[i])
    }

    /// Removes a key-value pair from the collection.
    pub fn remove(&mut self, key: &str) -> bool {
        if let Some(index) = self.hash.remove(key) {
            self.array.swap_remove(index);
            let moved_key = self.keys.swap_remove(index);
            // The last element was swapped into `index`; keep its hash entry
            // pointing at the new position (Phase 1 finding SE-1).
            if index < self.array.len() {
                self.hash.insert(moved_key, index);
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
        self.keys.clear();
    }
}

impl<V> Default for AssociativeArray<V> {
    fn default() -> Self {
        Self::new()
    }
}
