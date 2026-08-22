//! Ported from `packages/engine/Source/Core/DoubleEndedPriorityQueue.js`.
//!
//! Array-backed min-max heap implementation of a double-ended priority queue.

use crate::math::CesiumMath;
use std::sync::Arc;

/// A double-ended priority queue backed by a min-max heap.
pub struct DoubleEndedPriorityQueue<T: Clone> {
    comparator: Arc<dyn Fn(&T, &T) -> f64 + Send + Sync>,
    maximum_length: Option<usize>,
    array: Vec<Option<T>>,
    length: usize,
}

impl<T: Clone> DoubleEndedPriorityQueue<T> {
    /// Creates a new DoubleEndedPriorityQueue.
    pub fn new(
        comparator: impl Fn(&T, &T) -> f64 + Send + Sync + 'static,
        maximum_length: Option<usize>,
    ) -> Self {
        let array = match maximum_length {
            Some(ml) => vec![None; ml],
            None => Vec::new(),
        };
        Self {
            comparator: Arc::new(comparator),
            maximum_length,
            array,
            length: 0,
        }
    }

    /// Gets the number of elements.
    pub fn length(&self) -> usize {
        self.length
    }

    /// Gets the maximum length.
    pub fn maximum_length(&self) -> Option<usize> {
        self.maximum_length
    }

    /// Gets the internal array.
    pub fn internal_array(&self) -> &[Option<T>] {
        &self.array
    }

    /// Removes all elements.
    pub fn reset(&mut self) {
        self.length = 0;
        if let Some(ml) = self.maximum_length {
            for i in 0..ml {
                self.array[i] = None;
            }
        } else {
            self.array.clear();
        }
    }

    /// Resorts the queue.
    pub fn resort(&mut self) {
        let length = self.length;
        for i in 0..length {
            push_up(self, i);
        }
    }

    /// Inserts an element. Returns the removed minimum if at capacity.
    pub fn insert(&mut self, element: T) -> Option<T> {
        let mut removed_element = None;

        if let Some(max_len) = self.maximum_length {
            if max_len == 0 {
                return None;
            } else if self.length == max_len {
                if let Some(min_ref) = &self.array[0] {
                    if (self.comparator)(&element, min_ref) <= 0.0 {
                        return Some(element);
                    }
                }
                removed_element = self.remove_minimum();
            }
        }

        let index = self.length;
        if index < self.array.len() {
            self.array[index] = Some(element);
        } else {
            self.array.push(Some(element));
        }
        self.length += 1;
        push_up(self, index);

        removed_element
    }

    /// Removes and returns the minimum element.
    pub fn remove_minimum(&mut self) -> Option<T> {
        let length = self.length;
        if length == 0 {
            return None;
        }

        self.length -= 1;
        let minimum_element = self.array[0].take();

        if length >= 2 {
            self.array[0] = self.array[length - 1].take();
            push_down(self, 0);
        }

        self.array[length - 1] = None;
        minimum_element
    }

    /// Removes and returns the maximum element.
    pub fn remove_maximum(&mut self) -> Option<T> {
        let length = self.length;
        if length == 0 {
            return None;
        }

        self.length -= 1;
        let maximum_element;

        if length <= 2 {
            maximum_element = self.array[length - 1].take();
        } else {
            let max_idx = if greater_than(self, 1, 2) { 1 } else { 2 };
            maximum_element = self.array[max_idx].take();
            self.array[max_idx] = self.array[length - 1].take();
            if length >= 4 {
                push_down(self, max_idx);
            }
        }

        self.array[length - 1] = None;
        maximum_element
    }

    /// Gets a reference to the minimum element.
    pub fn get_minimum(&self) -> Option<&T> {
        if self.length == 0 {
            return None;
        }
        self.array[0].as_ref()
    }

    /// Gets a reference to the maximum element.
    pub fn get_maximum(&self) -> Option<&T> {
        let length = self.length;
        if length == 0 {
            return None;
        }
        if length <= 2 {
            return self.array[length - 1].as_ref();
        }
        let idx = if greater_than(self, 1, 2) { 1 } else { 2 };
        self.array[idx].as_ref()
    }
}

fn swap<T: Clone>(queue: &mut DoubleEndedPriorityQueue<T>, a: usize, b: usize) {
    queue.array.swap(a, b);
}

fn less_than<T: Clone>(queue: &DoubleEndedPriorityQueue<T>, a: usize, b: usize) -> bool {
    (queue.comparator)(
        queue.array[a].as_ref().unwrap(),
        queue.array[b].as_ref().unwrap(),
    ) < 0.0
}

fn greater_than<T: Clone>(queue: &DoubleEndedPriorityQueue<T>, a: usize, b: usize) -> bool {
    (queue.comparator)(
        queue.array[a].as_ref().unwrap(),
        queue.array[b].as_ref().unwrap(),
    ) > 0.0
}

fn push_up<T: Clone>(queue: &mut DoubleEndedPriorityQueue<T>, mut index: usize) {
    if index == 0 {
        return;
    }
    let on_min_level = (CesiumMath::log2((index + 1) as f64).floor() as usize) % 2 == 0;
    let parent_index = (index - 1) / 2;
    let less_than_parent = less_than(queue, index, parent_index);

    if less_than_parent != on_min_level {
        swap(queue, index, parent_index);
        index = parent_index;
    }

    while index >= 3 {
        let grandparent_index = (index - 3) / 4;
        if less_than(queue, index, grandparent_index) != less_than_parent {
            break;
        }
        swap(queue, index, grandparent_index);
        index = grandparent_index;
    }
}

fn push_down<T: Clone>(queue: &mut DoubleEndedPriorityQueue<T>, mut index: usize) {
    let length = queue.length;
    let on_min_level = (CesiumMath::log2((index + 1) as f64).floor() as usize) % 2 == 0;

    loop {
        let left_child_index = 2 * index + 1;
        if left_child_index >= length {
            break;
        }

        let mut target = left_child_index;
        let right_child_index = left_child_index + 1;
        if right_child_index < length {
            if less_than(queue, right_child_index, target) == on_min_level {
                target = right_child_index;
            }
            let grand_child_start = 2 * left_child_index + 1;
            let grand_child_count =
                usize::min(length.saturating_sub(grand_child_start), 4);
            for i in 0..grand_child_count {
                let gc_index = grand_child_start + i;
                if less_than(queue, gc_index, target) == on_min_level {
                    target = gc_index;
                }
            }
        }

        if less_than(queue, target, index) == on_min_level {
            swap(queue, target, index);
            if target != left_child_index && target != right_child_index {
                let parent_of_grandchild = (target - 1) / 2;
                if greater_than(queue, target, parent_of_grandchild) == on_min_level {
                    swap(queue, target, parent_of_grandchild);
                }
            }
        }

        index = target;
    }
}
