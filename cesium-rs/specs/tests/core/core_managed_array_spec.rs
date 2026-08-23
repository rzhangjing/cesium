use cesium_core::managed_array::ManagedArray;

#[test]
fn constructor_has_expected_default_values() {
    let array: ManagedArray<f64> = ManagedArray::new(0);
    assert_eq!(array.length(), 0);
}

#[test]
fn constructor_initializes_length() {
    let array = ManagedArray::<f64>::new(10);
    assert_eq!(array.length(), 10);
    assert_eq!(array.values().len(), 10);
}

#[test]
fn can_get_and_set_values() {
    let length = 10;
    let mut array = ManagedArray::<f64>::new(length);
    for i in 0..length {
        array.set(i, (i * i) as f64);
    }
    for i in 0..length {
        assert_eq!(*array.get(i), (i * i) as f64);
        assert_eq!(array.values()[i], (i * i) as f64);
    }
}

#[test]
fn set_resizes_array() {
    let mut array = ManagedArray::<f64>::new(0);
    array.set(0, 1.0);
    assert_eq!(array.length(), 1);
    array.set(5, 2.0);
    assert_eq!(array.length(), 6);
    array.set(2, 3.0);
    assert_eq!(array.length(), 6);
}

#[test]
fn peeks_at_the_last_element() {
    let mut array = ManagedArray::<f64>::new(0);
    assert!(array.peek().is_none());
    array.push(0.0);
    assert_eq!(*array.peek().unwrap(), 0.0);
    array.push(1.0);
    array.push(2.0);
    assert_eq!(*array.peek().unwrap(), 2.0);
}

#[test]
fn can_push_values() {
    let mut array = ManagedArray::<f64>::new(0);
    let length = 10;
    for i in 0..length {
        let val = i as f64 * 0.5;
        array.push(val);
        assert_eq!(array.length(), i + 1);
        assert_eq!(*array.get(i), val);
    }
}

#[test]
fn can_pop_values() {
    let length = 10;
    let mut array = ManagedArray::<f64>::new(length);
    for i in 0..length {
        array.set(i, i as f64 * 1.5);
    }
    for i in (0..length).rev() {
        let val = *array.get(i);
        assert_eq!(array.pop().unwrap(), val);
        assert_eq!(array.length(), i);
    }
}

#[test]
fn pop_returns_none_if_array_is_empty() {
    let mut array = ManagedArray::<f64>::new(0);
    array.push(1.0);
    assert_eq!(array.pop().unwrap(), 1.0);
    assert!(array.pop().is_none());
}

#[test]
fn reserve() {
    let mut array = ManagedArray::<f64>::new(2);
    // reserve expands internal capacity but logical length stays the same
    array.reserve(10);
    assert_eq!(array.length(), 2);
    array.reserve(20);
    assert_eq!(array.length(), 2);
    // reserve with smaller value should not shrink
    array.reserve(5);
    assert_eq!(array.length(), 2);
    // We can still set values beyond current length (set auto-resizes)
    array.set(2, 42.0);
    assert_eq!(array.length(), 3);
    assert_eq!(*array.get(2), 42.0);
}

#[test]
fn resize() {
    let mut array = ManagedArray::<f64>::new(2);
    array.resize(10);
    assert_eq!(array.values().len(), 10);
    assert_eq!(array.length(), 10);
    array.resize(20);
    assert_eq!(array.values().len(), 20);
    assert_eq!(array.length(), 20);
    array.resize(5);
    // values() returns only up to logical length
    assert_eq!(array.length(), 5);
}

#[test]
fn trim() {
    let mut array = ManagedArray::<f64>::new(2);
    array.reserve(10);
    assert_eq!(array.length(), 2);
    // After reserve, internal array is 10 long
    array.trim(None); // trim to logical length
    // Internal array should now be trimmed to 2
    array.trim(Some(5));
    assert_eq!(array.length(), 2);
    array.trim(Some(3));
    assert_eq!(array.length(), 2);
}
