//! DoublyLinkedList specs - ported from:
//! - packages/engine/Specs/Core/DoublyLinkedListSpec.js (16 it())
//!
//! A-class tests: 16 (node identity compared via Rc::ptr_eq)

use cesium_geospatial::doubly_linked_list::{DoublyLinkedList, NodeRef};
use std::rc::Rc;

/// Asserts that the list contains exactly `nodes` in order, verifying head/tail
/// and every node's next/previous pointers (mirrors the JS `expectOrder` helper).
fn expect_order(list: &DoublyLinkedList<i32>, nodes: &[NodeRef<i32>]) {
    let length = nodes.len();
    assert_eq!(list.length(), length);

    // Verify head and tail pointers.
    let head = list.head().expect("head should be defined");
    let tail = list.tail().expect("tail should be defined");
    assert!(Rc::ptr_eq(&head, &nodes[0]), "head mismatch");
    assert!(Rc::ptr_eq(&tail, &nodes[length - 1]), "tail mismatch");

    // Verify that the linked list has nodes in the expected order.
    let mut node = list.head();
    for i in 0..length {
        let expected = &nodes[i];
        let n = node.as_ref().expect("node should be defined");
        assert!(Rc::ptr_eq(n, expected), "node mismatch at index {}", i);

        let expected_next = if i == length - 1 {
            None
        } else {
            Some(nodes[i + 1].clone())
        };
        let expected_prev = if i == 0 {
            None
        } else {
            Some(nodes[i - 1].clone())
        };

        let actual_next = n.borrow().next.clone();
        let actual_prev = n.borrow().previous.clone();

        match (&actual_next, &expected_next) {
            (None, None) => {}
            (Some(a), Some(b)) => assert!(Rc::ptr_eq(a, b), "next mismatch at index {}", i),
            _ => panic!("next defined-ness mismatch at index {}", i),
        }
        match (&actual_prev, &expected_prev) {
            (None, None) => {}
            (Some(a), Some(b)) => {
                assert!(Rc::ptr_eq(a, b), "previous mismatch at index {}", i)
            }
            _ => panic!("previous defined-ness mismatch at index {}", i),
        }

        node = actual_next;
    }
}

#[test]
fn constructs() {
    let list: DoublyLinkedList<i32> = DoublyLinkedList::new();
    assert!(list.head().is_none());
    assert!(list.tail().is_none());
    assert_eq!(list.length(), 0);
}

#[test]
fn adds_items() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);

    // head/tail both point to the single node.
    assert!(Rc::ptr_eq(&list.head().unwrap(), &node));
    assert!(Rc::ptr_eq(&list.tail().unwrap(), &node));
    assert_eq!(list.length(), 1);

    assert_eq!(node.borrow().item, 1);
    assert!(node.borrow().previous.is_none());
    assert!(node.borrow().next.is_none());

    let node2 = list.add(2);

    assert!(Rc::ptr_eq(&list.head().unwrap(), &node));
    assert!(Rc::ptr_eq(&list.tail().unwrap(), &node2));
    assert_eq!(list.length(), 2);

    assert_eq!(node2.borrow().item, 2);
    assert!(Rc::ptr_eq(&node2.borrow().previous.clone().unwrap(), &node));
    assert!(node2.borrow().next.is_none());
    assert!(Rc::ptr_eq(&node.borrow().next.clone().unwrap(), &node2));

    let node3 = list.add(3);

    assert!(Rc::ptr_eq(&list.head().unwrap(), &node));
    assert!(Rc::ptr_eq(&list.tail().unwrap(), &node3));
    assert_eq!(list.length(), 3);

    assert_eq!(node3.borrow().item, 3);
    assert!(Rc::ptr_eq(&node3.borrow().previous.clone().unwrap(), &node2));
    assert!(node3.borrow().next.is_none());
    assert!(Rc::ptr_eq(&node2.borrow().next.clone().unwrap(), &node3));
}

#[test]
fn removes_from_a_list_with_one_item() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);

    list.remove(Some(&node));

    assert!(list.head().is_none());
    assert!(list.tail().is_none());
    assert_eq!(list.length(), 0);
}

#[test]
fn removes_head_of_list() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);
    let node2 = list.add(2);

    list.remove(Some(&node));

    assert!(Rc::ptr_eq(&list.head().unwrap(), &node2));
    assert!(Rc::ptr_eq(&list.tail().unwrap(), &node2));
    assert_eq!(list.length(), 1);
}

#[test]
fn removes_tail_of_list() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);
    let node2 = list.add(2);

    list.remove(Some(&node2));

    assert!(Rc::ptr_eq(&list.head().unwrap(), &node));
    assert!(Rc::ptr_eq(&list.tail().unwrap(), &node));
    assert_eq!(list.length(), 1);
}

#[test]
fn removes_middle_of_list() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);
    let node2 = list.add(2);
    let node3 = list.add(3);

    list.remove(Some(&node2));

    assert!(Rc::ptr_eq(&list.head().unwrap(), &node));
    assert!(Rc::ptr_eq(&list.tail().unwrap(), &node3));
    assert_eq!(list.length(), 2);
}

#[test]
fn removes_nothing() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);

    list.remove(None);

    assert!(Rc::ptr_eq(&list.head().unwrap(), &node));
    assert!(Rc::ptr_eq(&list.tail().unwrap(), &node));
    assert_eq!(list.length(), 1);
}

#[test]
fn splices_next_node_before_node() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);
    let node2 = list.add(2);
    let node3 = list.add(3);
    let node4 = list.add(4);
    let node5 = list.add(5);

    // Move node2 after node4.
    list.splice(&node4, &node2);
    expect_order(&list, &[node, node3, node4, node2, node5]);
}

#[test]
fn splices_next_node_after_node() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);
    let node2 = list.add(2);
    let node3 = list.add(3);
    let node4 = list.add(4);
    let node5 = list.add(5);

    // Move node4 after node2.
    list.splice(&node2, &node4);
    expect_order(&list, &[node, node2, node4, node3, node5]);
}

#[test]
fn splices_next_node_immediately_before_node() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);
    let node2 = list.add(2);
    let node3 = list.add(3);
    let node4 = list.add(4);

    // Move node2 after node3.
    list.splice(&node3, &node2);
    expect_order(&list, &[node, node3, node2, node4]);
}

#[test]
fn splices_next_node_immediately_after_node() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);
    let node2 = list.add(2);
    let node3 = list.add(3);
    let node4 = list.add(4);

    // node3 is already immediately after node2: order does not change.
    list.splice(&node2, &node3);
    expect_order(&list, &[node, node2, node3, node4]);
}

#[test]
fn splices_node_equal_to_next_node() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);
    let node2 = list.add(2);
    let node3 = list.add(3);

    // node === nextNode: order does not change.
    list.splice(&node2, &node2);
    expect_order(&list, &[node, node2, node3]);
}

#[test]
fn splices_when_next_node_was_tail() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);
    let node2 = list.add(2);
    let node3 = list.add(3);
    let node4 = list.add(4);

    list.splice(&node2, &node4);
    expect_order(&list, &[node, node2, node4, node3]);
}

#[test]
fn splices_when_node_was_tail() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);
    let node2 = list.add(2);
    let node3 = list.add(3);
    let node4 = list.add(4);

    list.splice(&node4, &node2);
    expect_order(&list, &[node, node3, node4, node2]);
}

#[test]
fn splices_when_next_node_was_head() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);
    let node2 = list.add(2);
    let node3 = list.add(3);
    let node4 = list.add(4);

    list.splice(&node3, &node);
    expect_order(&list, &[node2, node3, node, node4]);
}

#[test]
fn splices_when_node_was_head() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);
    let node2 = list.add(2);
    let node3 = list.add(3);
    let node4 = list.add(4);

    list.splice(&node, &node3);
    expect_order(&list, &[node, node3, node2, node4]);
}
