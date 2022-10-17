#![no_std]
#![forbid(unsafe_code)]
/// A doubly linked list using `Rc` and `RefCell`.
///  
/// Pros:
/// - No Dependencies
/// - Fully `no_std`
/// - Insert and remove data without copying
/// - No re-allocation required
///
/// Cons:
/// - Feature incomplete (`.iter()`, `.iter_mut()` and Cursors cant be implemented due to `Rc` and `RefCell`)
/// - `Rc` and `RefCell` are slow (reference counting and locking)
/// - `Rc` and `RefCell` are `!Send` and `!Sync` (but using `Arc` and `Mutex` would be even worse)
/// - We have to manually drop() pointers when removing nodes. If we forget, our program will panic.
/// - Allocates every node on the heap individually
///
/// This implementation is similar to https://rust-unofficial.github.io/too-many-lists/fourth.html
extern crate alloc;

use alloc::rc::Rc;
use core::cell::RefCell;

pub struct LinkedList<T> {
    len: usize,
    head_tail: Option<(NodeRef<T>, NodeRef<T>)>,
}

impl<T> LinkedList<T> {
    pub fn new() -> Self {
        Self {
            head_tail: None,
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn push_front(&mut self, value: T) {
        let new_head = Node::new(value);

        let head_tail = if let Some((head, tail)) = self.head_tail.take() {
            head.borrow_mut().prev = Some(new_head.clone());
            new_head.borrow_mut().next = Some(head);

            (new_head, tail)
        } else {
            (new_head.clone(), new_head)
        };

        self.head_tail = Some(head_tail);
        self.len += 1;
    }

    pub fn push_back(&mut self, value: T) {
        let new_tail = Node::new(value);

        let head_tail = if let Some((head, tail)) = self.head_tail.take() {
            tail.borrow_mut().next = Some(new_tail.clone());
            new_tail.borrow_mut().prev = Some(tail);

            (head, new_tail)
        } else {
            (new_tail.clone(), new_tail)
        };

        self.head_tail = Some(head_tail);
        self.len += 1;
    }

    pub fn pop_front(&mut self) -> Option<T> {
        let (head, tail) = self.head_tail.take()?;

        if Rc::ptr_eq(&head, &tail) {
            drop(tail);
            return Some(
                Rc::try_unwrap(head)
                    .ok()
                    .expect("tail to be dropped")
                    .into_inner()
                    .value,
            );
        }

        let new_head = head.borrow_mut().next.take().unwrap();
        drop(new_head.borrow_mut().prev.take().unwrap());

        self.head_tail = Some((new_head, tail));
        self.len -= 1;

        Some(
            Rc::try_unwrap(head)
                .ok()
                .expect("other_head to be dropped")
                .into_inner()
                .value,
        )
    }

    pub fn pop_back(&mut self) -> Option<T> {
        let (head, tail) = self.head_tail.take()?;

        if Rc::ptr_eq(&head, &tail) {
            // they are pointing to the same thing, but let's keep the symmetry
            drop(head);
            return Some(
                Rc::try_unwrap(tail)
                    .ok()
                    .expect("head to be dropped")
                    .into_inner()
                    .value,
            );
        }

        let new_tail = tail.borrow_mut().prev.take().unwrap();
        drop(new_tail.borrow_mut().next.take().unwrap());

        self.head_tail = Some((head, new_tail));
        self.len -= 1;

        Some(
            Rc::try_unwrap(tail)
                .ok()
                .expect("other_tail to be dropped")
                .into_inner()
                .value,
        )
    }

    pub fn clear(&mut self) {
        while self.pop_front().is_some() {}
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        while self.pop_front().is_some() {}
    }
}

struct Node<T> {
    value: T,
    prev: Option<NodeRef<T>>,
    next: Option<NodeRef<T>>,
}

impl<T> Node<T> {
    fn new(value: T) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Node {
            value,
            prev: None,
            next: None,
        }))
    }
}

type NodeRef<T> = Rc<RefCell<Node<T>>>;

pub struct IntoIter<T>(LinkedList<T>);

impl<T> IntoIterator for LinkedList<T> {
    type Item = T;

    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self)
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop_front()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn push_pop() {
        let mut list = LinkedList::new();

        list.push_front(1);
        list.push_front(2);

        assert_eq!(list.pop_front(), Some(2));
        assert_eq!(list.pop_front(), Some(1));
        assert_eq!(list.pop_front(), None);

        let mut list = LinkedList::new();

        list.push_back(1);
        list.push_back(2);

        assert_eq!(list.pop_back(), Some(2));
        assert_eq!(list.pop_back(), Some(1));
        assert_eq!(list.pop_back(), None);

        let mut list = LinkedList::new();

        list.push_back(1);
        list.push_back(2);

        assert_eq!(list.pop_front(), Some(1));
        assert_eq!(list.pop_front(), Some(2));
        assert_eq!(list.pop_front(), None);

        let mut list = LinkedList::new();

        list.push_front(1);
        list.push_front(2);

        assert_eq!(list.pop_back(), Some(1));
        assert_eq!(list.pop_back(), Some(2));
        assert_eq!(list.pop_back(), None);
    }

    #[derive(Default)]
    struct Big([usize; 32]);

    #[test]
    fn push_back_first_big() {
        let mut list = LinkedList::new();

        for _ in 0..500 {
            list.push_back(Big::default());
        }
    }
}
