//! Port of `Core/QueueSpec.js`.
use cesium_core::queue::Queue;

#[test]
fn enqueue_and_dequeue() {
    let mut q: Queue<i32> = Queue::new();
    q.enqueue(1);
    q.enqueue(2);
    q.enqueue(3);
    assert_eq!(q.dequeue(), Some(1));
    assert_eq!(q.dequeue(), Some(2));
    assert_eq!(q.dequeue(), Some(3));
}

#[test]
fn dequeue_empty_returns_none() {
    let mut q: Queue<i32> = Queue::new();
    assert_eq!(q.dequeue(), None);
}

#[test]
fn length_updates() {
    let mut q: Queue<&str> = Queue::new();
    assert_eq!(q.length(), 0);
    q.enqueue("a");
    assert_eq!(q.length(), 1);
    q.dequeue();
    assert_eq!(q.length(), 0);
}

#[test]
fn peek_returns_front() {
    let mut q: Queue<i32> = Queue::new();
    q.enqueue(1);
    q.enqueue(2);
    assert_eq!(q.peek(), Some(&1));
    assert_eq!(q.length(), 2);
}

#[test]
fn peek_empty_returns_none() {
    let q: Queue<i32> = Queue::new();
    assert_eq!(q.peek(), None);
}

#[test]
fn contains_works() {
    let mut q: Queue<i32> = Queue::new();
    q.enqueue(1);
    assert!(q.contains(&1));
    assert!(!q.contains(&2));
}

#[test]
fn clear_works() {
    let mut q: Queue<i32> = Queue::new();
    q.enqueue(1);
    q.enqueue(2);
    q.clear();
    assert_eq!(q.length(), 0);
}

#[test]
fn sort_works() {
    let mut q: Queue<i32> = Queue::new();
    q.enqueue(99);
    q.enqueue(6);
    q.enqueue(1);
    q.enqueue(53);
    q.enqueue(4);
    q.enqueue(0);

    q.dequeue(); // remove 99
    q.sort_by(|a, b| a.cmp(b));

    assert_eq!(q.dequeue(), Some(0));
    assert_eq!(q.dequeue(), Some(1));
    assert_eq!(q.dequeue(), Some(4));
    assert_eq!(q.dequeue(), Some(6));
    assert_eq!(q.dequeue(), Some(53));
}
