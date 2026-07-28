//! A heap data structure with a user-defined comparator.
//! Maps to CesiumJS `Core/Heap.js`

/// A heap that uses a comparator function to maintain the heap property.
/// The comparator should return a negative value if `a` has higher priority,
/// zero if equal, and positive if `b` has higher priority (min-heap by default).
pub struct Heap<T, F>
where
    F: Fn(&T, &T) -> std::cmp::Ordering,
{
    comparator: F,
    array: Vec<T>,
    maximum_length: Option<usize>,
}

impl<T, F> Heap<T, F>
where
    F: Fn(&T, &T) -> std::cmp::Ordering,
{
    /// Creates a new Heap with the given comparator.
    pub fn new(comparator: F) -> Self {
        Self {
            comparator,
            array: Vec::new(),
            maximum_length: None,
        }
    }

    /// Returns the number of elements in the heap.
    pub fn length(&self) -> usize {
        self.array.len()
    }

    /// Returns the maximum length constraint, if set.
    pub fn maximum_length(&self) -> Option<usize> {
        self.maximum_length
    }

    /// Sets the maximum length. If the current length exceeds this,
    /// excess elements are removed.
    ///
    /// # Panics
    /// Panics if `maximum_length` would be negative (not applicable for usize).
    pub fn set_maximum_length(&mut self, maximum_length: usize) {
        self.maximum_length = Some(maximum_length);
        if self.array.len() > maximum_length {
            self.array.truncate(maximum_length);
        }
    }

    /// Returns a reference to the internal array.
    pub fn internal_array(&self) -> &[T] {
        &self.array
    }

    /// Inserts a value into the heap.
    /// Returns the removed element if maximumLength was exceeded, otherwise None.
    pub fn insert(&mut self, value: T) -> Option<T> {
        let mut removed = None;

        if let Some(max_len) = self.maximum_length {
            if self.array.len() >= max_len {
                // Insert at end, bubble up, then remove the last (least priority)
                self.array.push(value);
                self.bubble_up(self.array.len() - 1);
                // The element to remove is the one with least priority (last after heapify)
                // In a min-heap, the max element is somewhere in the leaves.
                // CesiumJS approach: insert, then if over max, remove the last element
                // after bubbling. Actually CesiumJS inserts then pops the last from internal array.
                // Let's follow CesiumJS: insert normally, if length > maximumLength,
                // remove the element at the end of the internal array (which after bubble-up
                // is the one that was displaced).
                // Actually CesiumJS does: array[length] = value, length++, bubbleUp,
                // then if length > maximumLength: removed = array[--length], array.length = length
                // This means it removes the LAST element in the array (not the root).
                // After bubble-up, the newly inserted element is in its correct position,
                // and the last position holds whatever was displaced. But that's not necessarily
                // the least-priority element.
                //
                // Looking at CesiumJS source more carefully:
                // insert: this._array[this._length] = value; this._length++; bubbleUp;
                //         if defined maximumLength && this._length > maximumLength:
                //           removed = this._array[this._length - 1]; this._length--;
                //           this._array.length = this._length; (truncates)
                // So it removes the LAST element in the array after bubble-up.
                // After bubble-up, the new value has been moved to its correct position,
                // and whatever was swapped down is at the end. So the removed element
                // is the one that got pushed to the bottom during bubble-up.
                removed = self.array.pop();
            } else {
                self.array.push(value);
                self.bubble_up(self.array.len() - 1);
            }
        } else {
            self.array.push(value);
            self.bubble_up(self.array.len() - 1);
        }

        removed
    }

    /// Removes and returns the root (highest priority) element.
    /// Returns None if the heap is empty.
    pub fn pop(&mut self) -> Option<T> {
        if self.array.is_empty() {
            return None;
        }

        let last = self.array.len() - 1;
        self.array.swap(0, last);
        let result = self.array.pop();

        if !self.array.is_empty() {
            self.bubble_down(0);
        }

        result
    }

    /// Re-establishes the heap property after elements have been modified externally.
    pub fn resort(&mut self) {
        let len = self.array.len();
        if len <= 1 {
            return;
        }
        // Build heap from bottom up
        let mut i = len / 2;
        while i > 0 {
            i -= 1;
            self.bubble_down(i);
        }
    }

    fn bubble_up(&mut self, mut index: usize) {
        while index > 0 {
            let parent = (index - 1) / 2;
            if (self.comparator)(&self.array[index], &self.array[parent]) == std::cmp::Ordering::Less
            {
                self.array.swap(index, parent);
                index = parent;
            } else {
                break;
            }
        }
    }

    fn bubble_down(&mut self, mut index: usize) {
        let len = self.array.len();
        loop {
            let left = 2 * index + 1;
            let right = 2 * index + 2;
            let mut smallest = index;

            if left < len
                && (self.comparator)(&self.array[left], &self.array[smallest])
                    == std::cmp::Ordering::Less
            {
                smallest = left;
            }
            if right < len
                && (self.comparator)(&self.array[right], &self.array[smallest])
                    == std::cmp::Ordering::Less
            {
                smallest = right;
            }

            if smallest != index {
                self.array.swap(index, smallest);
                index = smallest;
            } else {
                break;
            }
        }
    }
}
