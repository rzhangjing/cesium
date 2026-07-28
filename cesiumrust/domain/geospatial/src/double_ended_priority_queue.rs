//! Maps to CesiumJS `Core/DoubleEndedPriorityQueue.js`
//!
//! Array-backed min-max heap implementation of a double-ended priority queue.
//! This data structure allows for efficient removal of minimum and maximum elements.

use std::cmp::Ordering;

/// Computes the level of a node in the complete binary tree:
/// `floor(log2(index + 1))`, using exact integer arithmetic to avoid
/// floating-point precision issues at powers of two.
#[inline]
fn level_of(index: usize) -> u32 {
    (index + 1).ilog2()
}

/// Array-backed min-max heap implementation of a double-ended priority queue.
///
/// The comparator returns `Ordering::Less` if `a` is lower priority than `b`
/// (mirrors CesiumJS `comparator(a, b) < 0`).
pub struct DoubleEndedPriorityQueue<T, F>
where
    F: Fn(&T, &T) -> Ordering,
{
    comparator: F,
    maximum_length: Option<usize>,
    /// The internal array. Slots beyond `length` are `None` (mirrors JS `undefined`).
    array: Vec<Option<T>>,
    length: usize,
}

impl<T, F> DoubleEndedPriorityQueue<T, F>
where
    F: Fn(&T, &T) -> Ordering,
{
    /// Creates a new double-ended priority queue.
    ///
    /// `maximum_length`: the maximum length of the queue. If an element is
    /// inserted when the queue is at full capacity, the minimum element is
    /// removed. `None` means the size of the queue is unlimited.
    pub fn new(comparator: F, maximum_length: Option<usize>) -> Self {
        let array = match maximum_length {
            Some(ml) => (0..ml).map(|_| None).collect(),
            None => Vec::new(),
        };
        Self {
            comparator,
            maximum_length,
            array,
            length: 0,
        }
    }

    /// Gets the number of elements in the queue.
    pub fn length(&self) -> usize {
        self.length
    }

    /// Gets the maximum number of elements in the queue, if set.
    pub fn maximum_length(&self) -> Option<usize> {
        self.maximum_length
    }

    /// Sets the maximum number of elements in the queue.
    /// If set to a smaller value than the current length, the lowest priority
    /// elements are removed. If set to `None`, the size of the queue is unlimited.
    pub fn set_maximum_length(&mut self, value: Option<usize>) {
        if let Some(value) = value {
            // Remove elements until the maximum length is met.
            while self.length > value {
                self.remove_minimum();
            }
            // The array size is fixed to the maximum length.
            self.array.resize_with(value, || None);
        }
        self.maximum_length = value;
    }

    /// Gets the internal array (slots beyond `length` are `None`).
    pub fn internal_array(&self) -> &[Option<T>] {
        &self.array
    }

    /// Gets a mutable reference to the internal array.
    pub fn internal_array_mut(&mut self) -> &mut [Option<T>] {
        &mut self.array
    }

    /// The comparator used by the queue.
    pub fn comparator(&self) -> &F {
        &self.comparator
    }

    /// Removes all elements from the queue.
    pub fn reset(&mut self) {
        self.length = 0;
        if self.maximum_length.is_some() {
            // Dereference all elements but keep the array the same size.
            for slot in self.array.iter_mut() {
                *slot = None;
            }
        } else {
            // Dereference all elements by clearing the array.
            self.array.clear();
        }
    }

    /// Resort the queue.
    pub fn resort(&mut self) {
        let length = self.length;
        // Fix the queue from the top-down.
        for i in 0..length {
            self.push_up(i);
        }
    }

    /// Inserts an element into the queue.
    /// If the queue is at full capacity, the minimum element is removed and returned.
    /// The new element is returned (and not added) if it is less than or equal
    /// priority to the minimum element.
    pub fn insert(&mut self, element: T) -> Option<T> {
        let mut removed_element = None;

        if let Some(maximum_length) = self.maximum_length {
            if maximum_length == 0 {
                return None;
            } else if self.length == maximum_length {
                // It's faster to access the minimum directly instead of calling
                // the getter because it avoids the length == 0 check.
                let minimum_element = self.array[0].as_ref().unwrap();
                if (self.comparator)(&element, minimum_element) != Ordering::Greater {
                    // The element being inserted is less than or equal to the
                    // minimum element, so don't insert anything and exit early.
                    return Some(element);
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
        self.push_up(index);

        removed_element
    }

    /// Removes the minimum element from the queue and returns it.
    /// If the queue is empty, the return value is `None`.
    pub fn remove_minimum(&mut self) -> Option<T> {
        let length = self.length;
        if length == 0 {
            return None;
        }

        self.length -= 1;

        // The minimum element is always the root.
        let minimum_element = self.array[0].take();

        if length >= 2 {
            self.array[0] = self.array[length - 1].take();
            self.push_down(0);
        }

        // Dereference removed element.
        self.array[length - 1] = None;

        minimum_element
    }

    /// Removes the maximum element from the queue and returns it.
    /// If the queue is empty, the return value is `None`.
    pub fn remove_maximum(&mut self) -> Option<T> {
        let length = self.length;
        if length == 0 {
            return None;
        }

        self.length -= 1;
        let maximum_element;

        // If the root has no children, the maximum is the root.
        // If the root has one child, the maximum is the child.
        if length <= 2 {
            maximum_element = self.array[length - 1].take();
        } else {
            // Otherwise, the maximum is the larger of the root's two children.
            let maximum_element_index = if self.greater_than(1, 2) { 1 } else { 2 };
            maximum_element = self.array[maximum_element_index].take();

            // Re-balance the heap.
            self.array[maximum_element_index] = self.array[length - 1].take();
            if length >= 4 {
                self.push_down(maximum_element_index);
            }
        }

        // Dereference removed element.
        self.array[length - 1] = None;

        maximum_element
    }

    /// Gets the minimum element in the queue, or `None` if empty.
    pub fn get_minimum(&self) -> Option<&T> {
        if self.length == 0 {
            return None;
        }
        // The minimum element is always the root.
        self.array[0].as_ref()
    }

    /// Gets the maximum element in the queue, or `None` if empty.
    pub fn get_maximum(&self) -> Option<&T> {
        let length = self.length;
        if length == 0 {
            return None;
        }
        // If the root has no children, the maximum is the root.
        // If the root has one child, the maximum is the child.
        if length <= 2 {
            return self.array[length - 1].as_ref();
        }
        // Otherwise, the maximum is the larger of the root's two children.
        self.array[if self.greater_than(1, 2) { 1 } else { 2 }].as_ref()
    }

    // Helper functions

    fn less_than(&self, index_a: usize, index_b: usize) -> bool {
        let a = self.array[index_a].as_ref().unwrap();
        let b = self.array[index_b].as_ref().unwrap();
        (self.comparator)(a, b) == Ordering::Less
    }

    fn greater_than(&self, index_a: usize, index_b: usize) -> bool {
        let a = self.array[index_a].as_ref().unwrap();
        let b = self.array[index_b].as_ref().unwrap();
        (self.comparator)(a, b) == Ordering::Greater
    }

    fn push_up(&mut self, mut index: usize) {
        if index == 0 {
            return;
        }
        let on_min_level = level_of(index) % 2 == 0;
        let parent_index = (index - 1) / 2;
        let less_than_parent = self.less_than(index, parent_index);

        // Get the element onto the correct level if it's not already.
        if less_than_parent != on_min_level {
            self.array.swap(index, parent_index);
            index = parent_index;
        }

        // Swap element with grandparent as long as it:
        // 1) has a grandparent
        // 2A) is less than the grandparent when on a min level
        // 2B) is greater than the grandparent when on a max level
        while index >= 3 {
            let grandparent_index = (index - 3) / 4;
            if self.less_than(index, grandparent_index) != less_than_parent {
                break;
            }
            self.array.swap(index, grandparent_index);
            index = grandparent_index;
        }
    }

    fn push_down(&mut self, mut index: usize) {
        let length = self.length;
        let on_min_level = level_of(index) % 2 == 0;

        // Loop as long as there is a left child.
        loop {
            let left_child_index = 2 * index + 1;
            if left_child_index >= length {
                break;
            }

            // Find the minimum (or maximum) child or grandchild.
            let mut target = left_child_index;
            let right_child_index = left_child_index + 1;
            if right_child_index < length {
                if self.less_than(right_child_index, target) == on_min_level {
                    target = right_child_index;
                }
                let grand_child_start = 2 * left_child_index + 1;
                let grand_child_count = if length > grand_child_start {
                    std::cmp::min(length - grand_child_start, 4)
                } else {
                    0
                };
                for i in 0..grand_child_count {
                    let grand_child_index = grand_child_start + i;
                    if self.less_than(grand_child_index, target) == on_min_level {
                        target = grand_child_index;
                    }
                }
            }

            // Swap the element into the correct spot.
            if self.less_than(target, index) == on_min_level {
                self.array.swap(target, index);
                if target != left_child_index && target != right_child_index {
                    let parent_of_grandchild_index = (target - 1) / 2;
                    if self.greater_than(target, parent_of_grandchild_index) == on_min_level {
                        self.array.swap(target, parent_of_grandchild_index);
                    }
                }
            }

            index = target;
        }
    }
}

impl<T, F> DoubleEndedPriorityQueue<T, F>
where
    T: Clone,
    F: Clone + Fn(&T, &T) -> Ordering,
{
    /// Clones the double ended priority queue.
    pub fn clone_queue(&self) -> Self {
        Self {
            comparator: self.comparator.clone(),
            maximum_length: self.maximum_length,
            array: self.array.clone(),
            length: self.length,
        }
    }
}
