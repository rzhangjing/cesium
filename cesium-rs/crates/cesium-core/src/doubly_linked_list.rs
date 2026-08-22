//! Ported from `packages/engine/Source/Core/DoublyLinkedList.js`.

/// A node in a doubly linked list.
pub struct DoublyLinkedListNode<T> {
    pub item: T,
    pub previous: Option<usize>,
    pub next: Option<usize>,
}

/// A doubly linked list.
pub struct DoublyLinkedList<T> {
    nodes: Vec<DoublyLinkedListNode<T>>,
    pub head: Option<usize>,
    pub tail: Option<usize>,
}

impl<T> DoublyLinkedList<T> {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            head: None,
            tail: None,
        }
    }

    pub fn length(&self) -> usize {
        self.nodes.len()
    }

    /// Adds the item to the end of the list.
    pub fn add(&mut self, item: T) -> usize {
        let index = self.nodes.len();
        let node = DoublyLinkedListNode {
            item,
            previous: self.tail,
            next: None,
        };
        self.nodes.push(node);

        if let Some(tail) = self.tail {
            self.nodes[tail].next = Some(index);
            self.tail = Some(index);
        } else {
            self.head = Some(index);
            self.tail = Some(index);
        }

        index
    }

    /// Gets a reference to the node at the given index.
    pub fn node(&self, index: usize) -> &DoublyLinkedListNode<T> {
        &self.nodes[index]
    }

    /// Gets a mutable reference to the node at the given index.
    pub fn node_mut(&mut self, index: usize) -> &mut DoublyLinkedListNode<T> {
        &mut self.nodes[index]
    }

    /// Removes the given node from the list.
    pub fn remove(&mut self, node_index: usize) {
        let prev = self.nodes[node_index].previous;
        let next = self.nodes[node_index].next;

        match (prev, next) {
            (Some(p), Some(n)) => {
                self.nodes[p].next = Some(n);
                self.nodes[n].previous = Some(p);
            }
            (Some(p), None) => {
                self.nodes[p].next = None;
                self.tail = Some(p);
            }
            (None, Some(n)) => {
                self.nodes[n].previous = None;
                self.head = Some(n);
            }
            (None, None) => {
                self.head = None;
                self.tail = None;
            }
        }

        self.nodes[node_index].next = None;
        self.nodes[node_index].previous = None;
    }
}

impl<T> Default for DoublyLinkedList<T> {
    fn default() -> Self {
        Self::new()
    }
}
