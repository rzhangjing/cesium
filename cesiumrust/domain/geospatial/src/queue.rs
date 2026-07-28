//! A FIFO queue data structure.
//! Maps to CesiumJS `Core/Queue.js`

use std::collections::VecDeque;

/// A FIFO queue with support for peek, contains, clear, and sort.
#[derive(Debug, Clone)]
pub struct Queue<T> {
    deque: VecDeque<T>,
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Queue<T> {
    /// Creates a new empty queue.
    pub fn new() -> Self {
        Self {
            deque: VecDeque::new(),
        }
    }

    /// Returns the number of elements in the queue.
    pub fn length(&self) -> usize {
        self.deque.len()
    }

    /// Adds an element to the back of the queue.
    pub fn enqueue(&mut self, item: T) {
        self.deque.push_back(item);
    }

    /// Removes and returns the element at the front of the queue.
    /// Returns None if the queue is empty.
    pub fn dequeue(&mut self) -> Option<T> {
        self.deque.pop_front()
    }

    /// Returns a reference to the element at the front of the queue without removing it.
    /// Returns None if the queue is empty.
    pub fn peek(&self) -> Option<&T> {
        self.deque.front()
    }

    /// Returns true if the queue contains the given item.
    pub fn contains(&self, item: &T) -> bool
    where
        T: PartialEq,
    {
        self.deque.contains(item)
    }

    /// Removes all elements from the queue.
    pub fn clear(&mut self) {
        self.deque.clear();
    }

    /// Sorts the elements in the queue using the given comparator.
    /// After sorting, the front of the queue has the "smallest" element.
    pub fn sort<F>(&mut self, comparator: F)
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        let mut vec: Vec<T> = self.deque.drain(..).collect();
        vec.sort_by(comparator);
        self.deque = VecDeque::from(vec);
    }
}
