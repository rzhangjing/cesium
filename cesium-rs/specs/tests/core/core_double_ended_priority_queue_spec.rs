use cesium_core::double_ended_priority_queue::DoubleEndedPriorityQueue;

fn min_comparator() -> impl Fn(&f64, &f64) -> f64 + Send + Sync + 'static {
    |a: &f64, b: &f64| a - b
}

#[test]
fn default_empty_queue() {
    let queue: DoubleEndedPriorityQueue<f64> = DoubleEndedPriorityQueue::new(min_comparator(), None);
    assert_eq!(queue.length(), 0);
    assert_eq!(queue.maximum_length(), None);
    assert!(queue.get_minimum().is_none());
    assert!(queue.get_maximum().is_none());
}

#[test]
fn insert_one_element() {
    let mut queue = DoubleEndedPriorityQueue::new(min_comparator(), None);
    queue.insert(1.0);
    assert_eq!(queue.length(), 1);
    assert_eq!(*queue.get_minimum().unwrap(), 1.0);
    assert_eq!(*queue.get_maximum().unwrap(), 1.0);
}

#[test]
fn insert_two_elements() {
    let mut queue = DoubleEndedPriorityQueue::new(min_comparator(), None);
    queue.insert(2.0);
    queue.insert(1.0);
    assert_eq!(queue.length(), 2);
    assert_eq!(*queue.get_minimum().unwrap(), 1.0);
    assert_eq!(*queue.get_maximum().unwrap(), 2.0);
}

#[test]
fn insert_three_elements() {
    let mut queue = DoubleEndedPriorityQueue::new(min_comparator(), None);
    queue.insert(2.0);
    queue.insert(1.0);
    queue.insert(3.0);
    assert_eq!(queue.length(), 3);
    assert_eq!(*queue.get_minimum().unwrap(), 1.0);
    assert_eq!(*queue.get_maximum().unwrap(), 3.0);
}

#[test]
fn remove_minimum() {
    let mut queue = DoubleEndedPriorityQueue::new(min_comparator(), None);
    queue.insert(3.0);
    queue.insert(1.0);
    queue.insert(2.0);
    let min = queue.remove_minimum().unwrap();
    assert_eq!(min, 1.0);
    assert_eq!(queue.length(), 2);
    assert_eq!(*queue.get_minimum().unwrap(), 2.0);
}

#[test]
fn remove_maximum() {
    let mut queue = DoubleEndedPriorityQueue::new(min_comparator(), None);
    queue.insert(3.0);
    queue.insert(1.0);
    queue.insert(2.0);
    let max = queue.remove_maximum().unwrap();
    assert_eq!(max, 3.0);
    assert_eq!(queue.length(), 2);
    assert_eq!(*queue.get_maximum().unwrap(), 2.0);
}

#[test]
fn limited_by_maximum_length() {
    let mut queue = DoubleEndedPriorityQueue::new(min_comparator(), Some(3));
    queue.insert(5.0);
    queue.insert(3.0);
    queue.insert(7.0);
    assert_eq!(queue.length(), 3);
    // Insert 1.0 which is less than minimum (3.0), should be returned as rejected
    let rejected = queue.insert(1.0);
    assert_eq!(rejected, Some(1.0));
    assert_eq!(queue.length(), 3);
    // Insert 4.0 which is greater than minimum, should evict minimum
    let evicted = queue.insert(4.0);
    assert_eq!(evicted, Some(3.0));
    assert_eq!(queue.length(), 3);
    assert_eq!(*queue.get_minimum().unwrap(), 4.0);
}

#[test]
fn reset_clears_queue() {
    let mut queue = DoubleEndedPriorityQueue::new(min_comparator(), None);
    queue.insert(1.0);
    queue.insert(2.0);
    queue.insert(3.0);
    queue.reset();
    assert_eq!(queue.length(), 0);
    assert!(queue.get_minimum().is_none());
}
