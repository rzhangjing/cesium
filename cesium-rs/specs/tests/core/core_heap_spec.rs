use cesium_core::heap::Heap;

/// Verify the heap property: parent <= children for a min-heap.
fn check_heap(heap: &Heap<f64>) -> bool {
    let array = heap.internal_array();
    let length = heap.length();
    for i in 0..length {
        let left = 2 * (i + 1) - 1;
        let right = 2 * (i + 1);
        if left < length {
            let parent = array[i].as_ref().unwrap();
            let child = array[left].as_ref().unwrap();
            if parent > child {
                return false;
            }
        }
        if right < length {
            let parent = array[i].as_ref().unwrap();
            let child = array[right].as_ref().unwrap();
            if parent > child {
                return false;
            }
        }
    }
    true
}

fn min_heap_comparator() -> impl Fn(&f64, &f64) -> f64 + Send + Sync + 'static {
    |a: &f64, b: &f64| a - b
}

#[test]
fn maintains_heap_property_on_insert() {
    let mut heap = Heap::new(min_heap_comparator());
    let values = [
        0.5, 0.3, 0.8, 0.1, 0.9, 0.2, 0.7, 0.4, 0.6, 0.0, 0.55, 0.35, 0.85, 0.15, 0.95,
        0.25, 0.75, 0.45, 0.65, 0.05,
    ];
    for &v in &values {
        heap.insert(v);
        assert!(check_heap(&heap), "heap property violated after inserting {v}");
    }
}

#[test]
fn maintains_heap_property_on_pop() {
    let mut heap = Heap::new(min_heap_comparator());
    let values = [
        0.5, 0.3, 0.8, 0.1, 0.9, 0.2, 0.7, 0.4, 0.6, 0.0, 0.55, 0.35, 0.85, 0.15, 0.95,
        0.25, 0.75, 0.45, 0.65, 0.05,
    ];
    for &v in &values {
        heap.insert(v);
    }
    for _ in 0..values.len() {
        heap.pop(0);
        assert!(check_heap(&heap), "heap property violated after pop");
    }
}

#[test]
fn limited_by_maximum_length() {
    let mut heap = Heap::new(min_heap_comparator());
    heap.set_maximum_length(10);
    let values = [
        0.5, 0.3, 0.8, 0.1, 0.9, 0.2, 0.7, 0.4, 0.6, 0.0, 0.55, 0.35, 0.85, 0.15, 0.95,
        0.25, 0.75, 0.45, 0.65, 0.05,
    ];
    for &v in &values {
        heap.insert(v);
        assert!(check_heap(&heap));
    }
    assert!(heap.length() <= 10);
}

#[test]
fn pops_in_sorted_order() {
    let mut heap = Heap::new(min_heap_comparator());
    let values = [
        0.5, 0.3, 0.8, 0.1, 0.9, 0.2, 0.7, 0.4, 0.6, 0.0,
    ];
    for &v in &values {
        heap.insert(v);
    }
    let mut prev = heap.pop(0).unwrap();
    for _ in 1..values.len() {
        let curr = heap.pop(0).unwrap();
        assert!(prev <= curr, "pop order: {} > {}", prev, curr);
        prev = curr;
    }
}

#[test]
fn insert_returns_removed_element_when_maximum_length_is_set() {
    let mut heap = Heap::new(min_heap_comparator());
    heap.set_maximum_length(5);

    // Fill to capacity
    for i in 0..5 {
        heap.insert(i as f64);
    }

    // Insert beyond capacity: new element enters, element at array[max_len] is ejected.
    // 100.0 bubbles up from index 5, but since it's the largest it stays at index 5
    // and gets immediately removed.
    let removed = heap.insert(100.0);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap(), 100.0);

    // Insert a small value: it bubbles up, displacing another element to the tail slot
    let removed = heap.insert(0.5);
    assert!(removed.is_some());
    // The exact ejected value depends on bubble-up positioning
    assert!(removed.unwrap() <= 4.0);
    assert_eq!(heap.length(), 5);
}

#[test]
fn resort() {
    #[derive(Clone)]
    struct Item {
        distance: f64,
        id: usize,
    }

    let mut heap = Heap::new(|a: &Item, b: &Item| a.distance - b.distance);

    // Insert 20 items with increasing distance
    for i in 0..20 {
        heap.insert(Item {
            distance: i as f64 / 19.0,
            id: i,
        });
    }

    // Pop all and verify sorted order
    let mut elements = Vec::new();
    let mut current_id = 0;
    while heap.length() > 0 {
        let element = heap.pop(0).unwrap();
        assert!(element.id >= current_id);
        current_id = element.id;
        elements.push(element);
    }

    // Rebuild with inverted distances and resort
    let mut heap2 = Heap::new(|a: &Item, b: &Item| a.distance - b.distance);
    for e in &elements {
        heap2.insert(Item {
            distance: 1.0 - e.distance,
            id: e.id,
        });
    }
    heap2.resort();

    // Pop and verify reverse order
    let mut current_id = usize::MAX;
    while heap2.length() > 0 {
        let element = heap2.pop(0).unwrap();
        assert!(element.id <= current_id);
        current_id = element.id;
    }
}

#[test]
fn setting_maximum_length_less_than_current_length_removes_elements() {
    let mut heap = Heap::new(min_heap_comparator());
    for i in 0..10 {
        heap.insert(i as f64);
    }
    assert_eq!(heap.length(), 10);
    heap.set_maximum_length(5);
    assert_eq!(heap.length(), 5);
}
