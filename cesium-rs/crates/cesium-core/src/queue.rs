//! Ported from `packages/engine/Source/Core/Queue.js`.

use std::collections::VecDeque;

/// A queue that can enqueue items at the end, and dequeue items from the front.
pub struct Queue<T> {
    deque: VecDeque<T>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self {
            deque: VecDeque::new(),
        }
    }

    /// The length of the queue.
    pub fn length(&self) -> usize {
        self.deque.len()
    }

    /// Enqueues the specified item.
    pub fn enqueue(&mut self, item: T) {
        self.deque.push_back(item);
    }

    /// Dequeues an item. Returns None if the queue is empty.
    pub fn dequeue(&mut self) -> Option<T> {
        self.deque.pop_front()
    }

    /// Returns the item at the front of the queue.
    pub fn peek(&self) -> Option<&T> {
        self.deque.front()
    }

    /// Check whether this queue contains the specified item.
    pub fn contains(&self, item: &T) -> bool
    where
        T: PartialEq,
    {
        self.deque.contains(item)
    }

    /// Remove all items from the queue.
    pub fn clear(&mut self) {
        self.deque.clear();
    }

    /// Sort the items in the queue in-place.
    pub fn sort_by(&mut self, compare: impl FnMut(&T, &T) -> std::cmp::Ordering) {
        let mut vec: Vec<T> = self.deque.drain(..).collect();
        vec.sort_by(compare);
        self.deque = vec.into();
    }
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self::new()
    }
}
