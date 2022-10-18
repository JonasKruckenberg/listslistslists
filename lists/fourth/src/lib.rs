#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

use ghost_cell::{GhostCell, GhostToken};
use typed_arena::Arena;

pub struct LinkedList<'arena, 'id, T> {
    len: usize,
    head_tail: Option<(NodeRef<'arena, 'id, T>, NodeRef<'arena, 'id, T>)>,
}

impl<'arena, 'id, T> LinkedList<'arena, 'id, T> {
    pub fn new() -> Self {
        Self {
            len: 0,
            head_tail: None
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn iter<'a>(&'a self, token: &'a GhostToken<'id>) -> Iter<'a, 'arena, 'id, T> {
        Iter {
            token,
            head_tail: self.head_tail,
        }
    }

    pub fn push_front(&mut self, value: T, arena: &'arena Arena<Node<'arena, 'id, T>>, token: &mut GhostToken<'id>) {
        let new_head = Self::insert(value, arena);

        let head_tail = if let Some((head, tail)) = self.head_tail.take() {
            head.borrow_mut(token).prev = Some(new_head);
            new_head.borrow_mut(token).next = Some(head);

            (new_head, tail)
        } else {
            (new_head, new_head)
        };

        self.head_tail = Some(head_tail);
    }

    pub fn push_back(&mut self, value: T, arena: &'arena Arena<Node<'arena, 'id, T>>, token: &mut GhostToken<'id>) {
        let new_tail = Self::insert(value, arena);

        let head_tail = if let Some((head, tail)) = self.head_tail.take() {
            tail.borrow_mut(token).next = Some(new_tail);
            new_tail.borrow_mut(token).prev = Some(tail);

            (head, new_tail)
        } else {
            (new_tail, new_tail)
        };

        self.head_tail = Some(head_tail);
    }

    // pub fn pop_front(&mut self) -> Option<T> {
    //     let (head, tail) = self.head_tail.take()?;

    //     if head == tail {
    //         return Some(self.remove(head).unwrap().value);
    //     }

    //     let new_head = self.get_mut(head).unwrap().next.take().unwrap();
    //     self.get_mut(new_head).unwrap().prev.take().unwrap();

    //     self.head_tail = Some((new_head, tail));
    //     self.len -= 1;

    //     Some(self.remove(head).unwrap().value)
    // }

    // pub fn pop_back(&mut self) -> Option<T> {
    //     let (head, tail) = self.head_tail.take()?;

    //     if head == tail {
    //         // they are pointing to the same thing, but let's keep the symmetry
    //         Some(self.remove(tail).unwrap().value);
    //     }

    //     let new_tail = self.get_mut(head).unwrap().prev.take().unwrap();
    //     self.get_mut(new_tail).unwrap().next.take().unwrap();

    //     self.head_tail = Some((head, new_tail));
    //     self.len -= 1;

    //     Some(self.remove(tail).unwrap().value)
    // }

    // pub fn clear(&mut self) {
    //     while self.pop_front().is_some() {}
    // }

    fn insert(value: T, arena: &'arena Arena<Node<'arena, 'id, T>>,) -> NodeRef<'arena, 'id, T> {
        GhostCell::from_mut(arena.alloc(Node {
            value,
            prev: None,
            next: None,
        }))
    }
}

pub struct Node<'arena, 'id, T> {
    value: T,
    prev: Option<NodeRef<'arena, 'id, T>>,
    next: Option<NodeRef<'arena, 'id, T>>,
}

pub type NodeRef<'arena, 'id, T> = &'arena GhostCell<'id, Node<'arena, 'id, T>>;

pub struct Iter<'a, 'arena, 'id, T> {
    token: &'a GhostToken<'id>,
    head_tail: Option<(NodeRef<'arena, 'id, T>, NodeRef<'arena, 'id, T>)>,
}

impl<'a, 'arena, 'id, T> Iterator for Iter<'a, 'arena, 'id, T> where 'arena: 'a {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let (head, tail) = self.head_tail.take()?;

        let node = head.borrow(self.token);

        if head.as_ptr() != tail.as_ptr() {
            self.head_tail = node.next.map(|n| (n, tail));
        } else {
            self.head_tail = None;
        }

        Some(&node.value)
    }
}

impl<'a, 'arena, 'id, T> DoubleEndedIterator for Iter<'a, 'arena, 'id, T> where 'arena: 'a {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (head, tail) = self.head_tail.take()?;

        let node = tail.borrow(self.token);

        if head.as_ptr() != tail.as_ptr() {
            self.head_tail = node.prev.map(|n| (head, n));
        } else {
            self.head_tail = None;
        }

        Some(&node.value)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn push_pop() {
        GhostToken::new(|ref mut token| {
            let arena = Arena::new();
            let mut list = LinkedList::new();

            list.push_front(1, &arena, token);
            list.push_front(2, &arena, token);

            // assert_eq!(list.pop_front(), Some(2));
            // assert_eq!(list.pop_front(), Some(1));
            // assert_eq!(list.pop_front(), None);
        });

        GhostToken::new(|ref mut token| {
            let arena = Arena::new();
            let mut list = LinkedList::new();

            list.push_back(1, &arena, token);
            list.push_back(2, &arena, token);

            // assert_eq!(list.pop_back(), Some(2));
            // assert_eq!(list.pop_back(), Some(1));
            // assert_eq!(list.pop_back(), None);
        });

        // GhostToken::new(|ref mut token| {
        //     let mut list = LinkedList::new();

        //     list.push_back(1, token);
        //     list.push_back(2, token);

        //     assert_eq!(list.pop_front(), Some(1));
        //     assert_eq!(list.pop_front(), Some(2));
        //     assert_eq!(list.pop_front(), None);
        // });

        // GhostToken::new(|ref mut token| {
        //     let mut list = LinkedList::new();

        //     list.push_front(1, token);
        //     list.push_front(2, token);

        //     assert_eq!(list.pop_back(), Some(1));
        //     assert_eq!(list.pop_back(), Some(2));
        //     assert_eq!(list.pop_back(), None);
        // });
    }

    #[test]
    pub fn iter() {
        GhostToken::new(|ref mut token| {
            let arena = Arena::new();
            let mut list = LinkedList::new();

            list.push_back(1, &arena, token);
            list.push_back(2, &arena, token);
            list.push_back(3, &arena, token);
            list.push_back(4, &arena, token);

            assert_eq!(list.iter(token).copied().collect::<Vec<_>>(), vec![1, 2, 3, 4])
        });
    }
}
