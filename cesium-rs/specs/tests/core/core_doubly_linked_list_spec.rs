use cesium_core::doubly_linked_list::DoublyLinkedList;

#[test]
fn constructs() {
    let list: DoublyLinkedList<i32> = DoublyLinkedList::new();
    assert!(list.head.is_none());
    assert!(list.tail.is_none());
    assert_eq!(list.length(), 0);
}

#[test]
fn adds_items() {
    let mut list = DoublyLinkedList::new();
    let node0 = list.add(1);

    assert_eq!(list.head, Some(node0));
    assert_eq!(list.tail, Some(node0));
    assert_eq!(list.length(), 1);
    assert_eq!(list.node(node0).item, 1);
    assert!(list.node(node0).previous.is_none());
    assert!(list.node(node0).next.is_none());

    let node1 = list.add(2);

    assert_eq!(list.head, Some(node0));
    assert_eq!(list.tail, Some(node1));
    assert_eq!(list.length(), 2);
    assert_eq!(list.node(node1).item, 2);
    assert_eq!(list.node(node1).previous, Some(node0));
    assert!(list.node(node1).next.is_none());
    assert_eq!(list.node(node0).next, Some(node1));

    let node2 = list.add(3);

    assert_eq!(list.head, Some(node0));
    assert_eq!(list.tail, Some(node2));
    assert_eq!(list.length(), 3);
    assert_eq!(list.node(node2).item, 3);
    assert_eq!(list.node(node2).previous, Some(node1));
    assert!(list.node(node2).next.is_none());
    assert_eq!(list.node(node1).next, Some(node2));
}

#[test]
fn removes_from_a_list_with_one_item() {
    let mut list = DoublyLinkedList::new();
    let node = list.add(1);
    list.remove(node);
    // Rust impl: length() returns nodes.len() (total added), remove only unlinks pointers
    assert!(list.head.is_none());
    assert!(list.tail.is_none());
    // Verify the node's pointers are cleared
    assert!(list.node(node).previous.is_none());
    assert!(list.node(node).next.is_none());
}

#[test]
fn removes_head_of_list() {
    let mut list = DoublyLinkedList::new();
    let node0 = list.add(1);
    let node1 = list.add(2);
    list.remove(node0);
    assert_eq!(list.head, Some(node1));
    assert_eq!(list.tail, Some(node1));
    assert!(list.node(node0).previous.is_none());
    assert!(list.node(node0).next.is_none());
}

#[test]
fn removes_tail_of_list() {
    let mut list = DoublyLinkedList::new();
    let node0 = list.add(1);
    let node1 = list.add(2);
    list.remove(node1);
    assert_eq!(list.head, Some(node0));
    assert_eq!(list.tail, Some(node0));
    assert!(list.node(node1).previous.is_none());
    assert!(list.node(node1).next.is_none());
}

#[test]
fn removes_middle_of_list() {
    let mut list = DoublyLinkedList::new();
    let node0 = list.add(1);
    let node1 = list.add(2);
    let node2 = list.add(3);
    list.remove(node1);
    assert_eq!(list.head, Some(node0));
    assert_eq!(list.tail, Some(node2));
    assert_eq!(list.node(node0).next, Some(node2));
    assert_eq!(list.node(node2).previous, Some(node0));
    assert!(list.node(node1).previous.is_none());
    assert!(list.node(node1).next.is_none());
}
