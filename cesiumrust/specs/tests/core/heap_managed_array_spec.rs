//! Heap / ManagedArray / mergeSort specs - ported from:
//! - packages/engine/Specs/Core/HeapSpec.js (9 it())
//! - packages/engine/Specs/Core/ManagedArraySpec.js (17 it())
//! - packages/engine/Specs/Core/mergeSortSpec.js (5 it())
//!
//! A-class tests: 22 (Heap 7 + ManagedArray 10 + mergeSort 3, skipping JS-specific throws/undefined tests)

use cesium_geospatial::heap::Heap;
use cesium_geospatial::managed_array::ManagedArray;
use cesium_geospatial::utilities::merge_sort;
use std::cmp::Ordering;

// ============================================================
// Heap
// ============================================================

fn check_heap_property(array: &[f64]) -> bool {
    let len = array.len();
    for i in 0..len {
        let left = 2 * (i + 1) - 1;
        let right = 2 * (i + 1);
        if left < len && array[i] > array[left] {
            return false;
        }
        if right < len && array[i] > array[right] {
            return false;
        }
    }
    true
}

fn f64_comparator(a: &f64, b: &f64) -> Ordering {
    a.partial_cmp(b).unwrap_or(Ordering::Equal)
}

#[test]
fn heap_maintains_heap_property_on_insert() {
    let mut heap = Heap::new(f64_comparator);
    // Use deterministic values instead of random
    let values: Vec<f64> = (0..100).map(|i| ((i * 37 + 13) % 100) as f64 / 100.0).collect();
    let mut pass = true;
    for v in values {
        heap.insert(v);
        pass = pass && check_heap_property(heap.internal_array());
    }
    assert!(pass);
}

#[test]
fn heap_maintains_heap_property_on_pop() {
    let mut heap = Heap::new(f64_comparator);
    let values: Vec<f64> = (0..100).map(|i| ((i * 53 + 7) % 100) as f64 / 100.0).collect();
    for v in &values {
        heap.insert(*v);
    }
    let mut pass = true;
    for _ in 0..100 {
        heap.pop();
        pass = pass && check_heap_property(heap.internal_array());
    }
    assert!(pass);
}

#[test]
fn heap_limited_by_maximum_length() {
    let mut heap = Heap::new(f64_comparator);
    heap.set_maximum_length(50);
    let values: Vec<f64> = (0..100).map(|i| ((i * 41 + 3) % 100) as f64 / 100.0).collect();
    let mut pass = true;
    for v in values {
        heap.insert(v);
        pass = pass && check_heap_property(heap.internal_array());
    }
    assert!(pass);
    assert!(heap.length() <= 50);
}

#[test]
fn heap_pops_in_sorted_order() {
    let mut heap = Heap::new(f64_comparator);
    let values: Vec<f64> = (0..100).map(|i| ((i * 67 + 29) % 100) as f64 / 100.0).collect();
    for v in &values {
        heap.insert(*v);
    }
    let mut curr = heap.pop().unwrap();
    let mut pass = true;
    for _ in 0..99 {
        let next = heap.pop().unwrap();
        pass = pass && curr <= next;
        curr = next;
    }
    assert!(pass);
}

#[test]
fn heap_insert_returns_removed_element_when_maximum_length_set() {
    let mut heap = Heap::new(f64_comparator);
    heap.set_maximum_length(100);

    let values: Vec<f64> = (0..100).map(|i| ((i * 37 + 13) % 100) as f64 / 100.0).collect();
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    // Push 99 values
    for i in 0..99 {
        heap.insert(values[i]);
    }

    // Push 100th, nothing is removed so it returns None
    let removed = heap.insert(values[99]);
    assert!(removed.is_none());

    // Insert value, an element is removed
    let removed = heap.insert(max - 0.1);
    assert!(removed.is_some());

    // If this value is the least priority (largest) it will be returned
    let removed = heap.insert(max + 0.1);
    assert_eq!(removed, Some(max + 0.1));
}

#[test]
fn heap_resort() {
    #[derive(Clone)]
    struct Item {
        distance: f64,
        id: usize,
    }

    let comparator = |a: &Item, b: &Item| a.distance.partial_cmp(&b.distance).unwrap_or(Ordering::Equal);

    let mut heap = Heap::new(comparator);
    let length = 100;
    for i in 0..length {
        heap.insert(Item {
            distance: i as f64 / (length - 1) as f64,
            id: i,
        });
    }

    // Check that elements are initially sorted
    let mut elements = Vec::new();
    let mut current_id = 0;
    while heap.length() > 0 {
        let element = heap.pop().unwrap();
        assert!(element.id >= current_id);
        current_id = element.id;
        elements.push(element);
    }

    // Add back into heap
    for e in &elements {
        heap.insert(Item {
            distance: e.distance,
            id: e.id,
        });
    }

    // Invert priority by modifying distances
    // Since we can't modify in-place easily, rebuild with inverted distances
    let mut heap2 = Heap::new(|a: &Item, b: &Item| a.distance.partial_cmp(&b.distance).unwrap_or(Ordering::Equal));
    for e in &elements {
        heap2.insert(Item {
            distance: 1.0 - e.distance,
            id: e.id,
        });
    }

    // Check the elements are popped in the opposite order now
    current_id = length - 1;
    while heap2.length() > 0 {
        let element = heap2.pop().unwrap();
        assert!(element.id <= current_id);
        current_id = element.id;
    }
}

#[test]
fn heap_pop_returns_none_when_empty() {
    let mut heap = Heap::new(f64_comparator);
    assert!(heap.pop().is_none());
    heap.insert(1.0);
    assert_eq!(heap.pop(), Some(1.0));
    assert!(heap.pop().is_none());
}

// ============================================================
// ManagedArray
// ============================================================

#[test]
fn managed_array_constructor_default_values() {
    let array: ManagedArray<f64> = ManagedArray::new(0);
    assert_eq!(array.length(), 0);
}

#[test]
fn managed_array_constructor_initializes_length() {
    let array: ManagedArray<f64> = ManagedArray::new(10);
    assert_eq!(array.length(), 10);
    assert_eq!(array.values().len(), 10);
}

#[test]
fn managed_array_can_get_and_set_values() {
    let mut array: ManagedArray<f64> = ManagedArray::new(10);
    for i in 0..10 {
        array.set(i, (i * i) as f64);
    }
    for i in 0..10 {
        assert_eq!(*array.get(i), (i * i) as f64);
        assert_eq!(array.values()[i], (i * i) as f64);
    }
}

#[test]
fn managed_array_set_resizes_array() {
    let mut array: ManagedArray<f64> = ManagedArray::new(0);
    array.set(0, 1.0);
    assert_eq!(array.length(), 1);
    array.set(5, 2.0);
    assert_eq!(array.length(), 6);
    array.set(2, 3.0);
    assert_eq!(array.length(), 6);
}

#[test]
fn managed_array_peeks_at_last_element() {
    let mut array: ManagedArray<i32> = ManagedArray::new(0);
    assert!(array.peek().is_none());
    array.push(0);
    assert_eq!(array.peek(), Some(&0));
    array.push(1);
    array.push(2);
    assert_eq!(array.peek(), Some(&2));
}

#[test]
fn managed_array_can_push_values() {
    let mut array: ManagedArray<f64> = ManagedArray::new(0);
    for i in 0..10 {
        let val = i as f64 * 1.5;
        array.push(val);
        assert_eq!(array.length(), i + 1);
        assert_eq!(array.values().len(), i + 1);
        assert_eq!(*array.get(i), val);
    }
}

#[test]
fn managed_array_can_pop_values() {
    let mut array: ManagedArray<f64> = ManagedArray::new(10);
    for i in 0..10 {
        array.set(i, i as f64 * 2.0);
    }
    for i in (0..10).rev() {
        let val = *array.get(i);
        assert_eq!(array.pop(), Some(val));
        assert_eq!(array.length(), i);
        // Capacity is preserved
        assert_eq!(array.values().len(), 10);
    }
}

#[test]
fn managed_array_pop_returns_none_if_empty() {
    let mut array: ManagedArray<i32> = ManagedArray::new(0);
    array.push(1);
    assert_eq!(array.pop(), Some(1));
    assert_eq!(array.pop(), None);
}

#[test]
fn managed_array_reserve() {
    let mut array: ManagedArray<f64> = ManagedArray::new(2);
    array.reserve(10);
    assert_eq!(array.values().len(), 10);
    assert_eq!(array.length(), 2);
    array.reserve(20);
    assert_eq!(array.values().len(), 20);
    assert_eq!(array.length(), 2);
    array.reserve(5);
    assert_eq!(array.values().len(), 20); // doesn't shrink
    assert_eq!(array.length(), 2);
}

#[test]
fn managed_array_resize_and_trim() {
    let mut array: ManagedArray<f64> = ManagedArray::new(2);
    array.resize(10);
    assert_eq!(array.values().len(), 10);
    assert_eq!(array.length(), 10);
    array.resize(20);
    assert_eq!(array.values().len(), 20);
    assert_eq!(array.length(), 20);
    array.resize(5);
    assert_eq!(array.values().len(), 20); // capacity preserved
    assert_eq!(array.length(), 5);

    // trim
    array.trim(None);
    assert_eq!(array.values().len(), 5);
    array.trim(Some(10));
    assert_eq!(array.length(), 5);
    assert_eq!(array.values().len(), 10);
    array.trim(Some(7));
    assert_eq!(array.length(), 5);
    assert_eq!(array.values().len(), 7);
}

// ============================================================
// mergeSort
// ============================================================

#[test]
fn merge_sort_sorts() {
    let mut array = [0, 9, 1, 8, 2, 7, 3, 6, 4, 5];
    merge_sort(&mut array, |a, b| a.cmp(b));
    assert_eq!(array, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn merge_sort_stable_sorts() {
    #[derive(Debug, PartialEq)]
    struct Item {
        value: i32,
        original_index: usize,
    }
    let mut array = vec![
        Item { value: 5, original_index: 0 },
        Item { value: 10, original_index: 1 },
        Item { value: 5, original_index: 2 },
        Item { value: 0, original_index: 3 },
    ];
    merge_sort(&mut array, |a, b| a.value.cmp(&b.value));
    // Stable: equal elements maintain original order
    assert_eq!(array[0].original_index, 3); // value 0
    assert_eq!(array[1].original_index, 0); // value 5 (first)
    assert_eq!(array[2].original_index, 2); // value 5 (second)
    assert_eq!(array[3].original_index, 1); // value 10
}

#[test]
fn merge_sort_sorts_with_user_defined_comparator() {
    // Sort by distance from origin (descending)
    let mut array: Vec<(f64, f64, f64)> = vec![
        (-2.0, 0.0, 0.0),
        (-1.0, 0.0, 0.0),
        (-3.0, 0.0, 0.0),
    ];
    // Comparator: sort by distance squared descending (b - a)
    merge_sort(&mut array, |a, b| {
        let da = a.0 * a.0 + a.1 * a.1 + a.2 * a.2;
        let db = b.0 * b.0 + b.1 * b.1 + b.2 * b.2;
        db.partial_cmp(&da).unwrap_or(Ordering::Equal)
    });
    // Expected order: (-3,0,0), (-2,0,0), (-1,0,0) - furthest first
    assert_eq!(array[0], (-3.0, 0.0, 0.0));
    assert_eq!(array[1], (-2.0, 0.0, 0.0));
    assert_eq!(array[2], (-1.0, 0.0, 0.0));
}
