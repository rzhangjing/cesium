//! Maps to CesiumJS `Core/DoublyLinkedList.js`
//!
//! A doubly linked list. Nodes are shared via `Rc<RefCell<_>>` so that callers
//! can hold handles to nodes (mirroring CesiumJS object references) and compare
//! them by identity.

use std::cell::RefCell;
use std::rc::Rc;

/// A shared reference to a doubly linked list node.
pub type NodeRef<T> = Rc<RefCell<DoublyLinkedListNode<T>>>;

/// A node in the doubly linked list.
pub struct DoublyLinkedListNode<T> {
    pub item: T,
    pub previous: Option<NodeRef<T>>,
    pub next: Option<NodeRef<T>>,
}

/// A doubly linked list.
pub struct DoublyLinkedList<T> {
    head: Option<NodeRef<T>>,
    tail: Option<NodeRef<T>>,
    length: usize,
}

impl<T> Default for DoublyLinkedList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> DoublyLinkedList<T> {
    /// Creates a new, empty doubly linked list.
    pub fn new() -> Self {
        Self {
            head: None,
            tail: None,
            length: 0,
        }
    }

    /// Gets the number of nodes in the list.
    pub fn length(&self) -> usize {
        self.length
    }

    /// Gets the head node, if any.
    pub fn head(&self) -> Option<NodeRef<T>> {
        self.head.clone()
    }

    /// Gets the tail node, if any.
    pub fn tail(&self) -> Option<NodeRef<T>> {
        self.tail.clone()
    }

    /// Adds the item to the end of the list, returning the new node.
    pub fn add(&mut self, item: T) -> NodeRef<T> {
        let node = Rc::new(RefCell::new(DoublyLinkedListNode {
            item,
            previous: self.tail.clone(),
            next: None,
        }));

        if let Some(tail) = &self.tail {
            tail.borrow_mut().next = Some(node.clone());
            self.tail = Some(node.clone());
        } else {
            self.head = Some(node.clone());
            self.tail = Some(node.clone());
        }

        self.length += 1;

        node
    }

    /// Removes the given node from the list. Does nothing if `node` is `None`
    /// (mirrors CesiumJS `remove(undefined)`).
    pub fn remove(&mut self, node: Option<&NodeRef<T>>) {
        if let Some(node) = node {
            remove_node(self, node);
            self.length -= 1;
        }
    }

    /// Moves `next_node` after `node`.
    pub fn splice(&mut self, node: &NodeRef<T>, next_node: &NodeRef<T>) {
        if Rc::ptr_eq(node, next_node) {
            return;
        }

        // Remove next_node, then insert after node.
        remove_node(self, next_node);

        let old_node_next = node.borrow().next.clone();
        node.borrow_mut().next = Some(next_node.clone());

        // next_node is the new tail if node was the tail.
        let node_is_tail = self
            .tail
            .as_ref()
            .map_or(false, |tail| Rc::ptr_eq(tail, node));
        if node_is_tail {
            self.tail = Some(next_node.clone());
        } else if let Some(old_next) = &old_node_next {
            old_next.borrow_mut().previous = Some(next_node.clone());
        }

        next_node.borrow_mut().next = old_node_next;
        next_node.borrow_mut().previous = Some(node.clone());
    }
}

fn remove_node<T>(list: &mut DoublyLinkedList<T>, node: &NodeRef<T>) {
    let previous = node.borrow().previous.clone();
    let next = node.borrow().next.clone();

    if let (Some(prev), Some(next)) = (&previous, &next) {
        prev.borrow_mut().next = Some(next.clone());
        next.borrow_mut().previous = Some(prev.clone());
    } else if let Some(prev) = &previous {
        // Remove last node.
        prev.borrow_mut().next = None;
        list.tail = Some(prev.clone());
    } else if let Some(next) = &next {
        // Remove first node.
        next.borrow_mut().previous = None;
        list.head = Some(next.clone());
    } else {
        // Remove the only node in the list.
        list.head = None;
        list.tail = None;
    }

    node.borrow_mut().next = None;
    node.borrow_mut().previous = None;
}
