#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

use ghost_cell::{GhostCell, GhostToken};
use static_rc::StaticRcRef;
use typed_arena::Arena;

pub struct LinkedList<'arena, 'id, T> {
    arena: &'arena Arena<Node<'arena, 'id, T>>,
    head_tail: Option<(HalfNodePtr<'arena, 'id, T>, HalfNodePtr<'arena, 'id, T>)>,
}

impl<'arena, 'id, T> LinkedList<'arena, 'id, T> {
    pub fn new(arena: &'arena Arena<Node<'arena, 'id, T>>) -> Self {
        Self {
            head_tail: None,
            arena,
        }
    }

    pub fn len(&self, token: &GhostToken<'id>) -> usize {
        self.iter(token).count()
    }

    pub fn is_empty(&self) -> bool {
        self.head_tail.is_none()
    }

    pub fn iter<'a>(&'a self, token: &'a GhostToken<'id>) -> Iter<'a, 'arena, 'id, T> {
        let head_tail = self.head_tail.as_ref().map(|head_tail| (&*head_tail.0, &*head_tail.1));

        Iter {
            token,
            head_tail,
        }
    }

    pub fn push_front(&mut self, value: T, token: &mut GhostToken<'id>) {
        let (one, two) = self.new_halves(value);

        let head_tail = if let Some((head, tail)) = self.head_tail.take() {
            head.borrow_mut(token).prev = Some(one);
            two.borrow_mut(token).next = Some(head);

            (two, tail)
        } else {
            (one, two)
        };

        self.head_tail = Some(head_tail)
    }

    pub fn push_back(&mut self, value: T, token: &mut GhostToken<'id>) {
        let (one, two) = self.new_halves(value);

        let head_tail = if let Some((head, tail)) = self.head_tail.take() {
            tail.borrow_mut(token).next = Some(one);
            two.borrow_mut(token).prev = Some(tail);

            (head, two)
        } else {
            (one, two)
        };

        self.head_tail = Some(head_tail)
    }

    pub fn pop_front(&mut self, token: &mut GhostToken<'id>) -> Option<T> {
        let (head, tail) = self.head_tail.take()?;

        // when there is only one element in the list
        if StaticRcRef::as_ptr(&head) == StaticRcRef::as_ptr(&tail) {
            return Some(Self::into_inner(head, tail, token));
        }

        let next = head.borrow_mut(token).next.take().unwrap();
        let other_head = next.borrow_mut(token).prev.take().unwrap();

        self.head_tail = Some((next, tail));

        Some(Self::into_inner(head, other_head, token))
    }

    pub fn pop_back(&mut self, token: &mut GhostToken<'id>) -> Option<T> {
        let (head, tail) = self.head_tail.take()?;

        // when there is only one element in the list
        if StaticRcRef::as_ptr(&head) == StaticRcRef::as_ptr(&tail) {
            return Some(Self::into_inner(head, tail, token));
        }

        let prev = tail
            .borrow_mut(token)
            .prev
            .take()
            .expect("Non-head should have a left node");
        let other_tail = prev
            .borrow_mut(token)
            .next
            .take()
            .expect("Non-tail should have a right node");

        self.head_tail = Some((head, prev));

        Some(Self::into_inner(tail, other_tail, token))
    }

    pub fn clear(&mut self, token: &mut GhostToken<'id>) {
        while self.pop_back(token).is_some() {}
    }

    fn new_halves(&self, value: T) -> (HalfNodePtr<'arena, 'id, T>, HalfNodePtr<'arena, 'id, T>) {
        let node = self.arena.alloc(Node {
            value: Some(value),
            prev: None,
            next: None,
        });

        let full = FullNodePtr::new(GhostNode::from_mut(node));

        StaticRcRef::split::<1, 1>(full)
    }

    fn into_inner(
        left: HalfNodePtr<'arena, 'id, T>,
        right: HalfNodePtr<'arena, 'id, T>,
        token: &mut GhostToken<'id>,
    ) -> T {
        let full = FullNodePtr::join(left, right);
        let ghost_cell = FullNodePtr::into_inner(full);
        let node = ghost_cell.borrow_mut(token);

        //  If the node still has a prev and next, they are leaked.
        debug_assert!(node.prev.is_none());
        debug_assert!(node.next.is_none());

        node.value.take().unwrap()
    }
}

pub struct Node<'arena, 'id, T> {
    value: Option<T>,
    prev: Option<HalfNodePtr<'arena, 'id, T>>,
    next: Option<HalfNodePtr<'arena, 'id, T>>,
}

type GhostNode<'arena, 'id, T> = GhostCell<'id, Node<'arena, 'id, T>>;

type HalfNodePtr<'arena, 'id, T> = StaticRcRef<'arena, GhostNode<'arena, 'id, T>, 1, 2>;
type FullNodePtr<'arena, 'id, T> = StaticRcRef<'arena, GhostNode<'arena, 'id, T>, 2, 2>;

pub struct Iter<'a, 'arena, 'id, T> {
    token: &'a GhostToken<'id>,
    head_tail: Option<(&'a GhostNode<'arena, 'id, T>, &'a GhostNode<'arena, 'id, T>)>,
}

impl<'a, 'arena, 'id, T> Iterator for Iter<'a, 'arena, 'id, T>
where
    'arena: 'a,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let (head, tail) = self.head_tail.take()?;

        let node = head.borrow(self.token);

        if head.as_ptr() != tail.as_ptr() {
            self.head_tail = node.next.as_ref().map(|n| (n.as_ref(), tail));
        } else {
            self.head_tail = None;
        }

        Some(node.value.as_ref().unwrap())
    }
}

impl<'a, 'arena, 'id, T> DoubleEndedIterator for Iter<'a, 'arena, 'id, T>
where
    'arena: 'a,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let (head, tail) = self.head_tail.take()?;

        let node = tail.borrow(self.token);

        if head.as_ptr() != tail.as_ptr() {
            self.head_tail = node.prev.as_ref().map(|n| (head, n.as_ref()));
        } else {
            self.head_tail = None;
        }

        Some(node.value.as_ref().unwrap())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn push_pop() {
        GhostToken::new(|ref mut token| {
            let arena = Arena::new();
            let mut list = LinkedList::new(&arena);

            list.push_front(1, token);
            list.push_front(2, token);

            assert_eq!(list.pop_front(token), Some(2));
            assert_eq!(list.pop_front(token), Some(1));
            assert_eq!(list.pop_front(token), None);
        });

        GhostToken::new(|ref mut token| {
            let arena = Arena::new();
            let mut list = LinkedList::new(&arena);

            list.push_back(1, token);
            list.push_back(2, token);

            assert_eq!(list.pop_back(token), Some(2));
            assert_eq!(list.pop_back(token), Some(1));
            assert_eq!(list.pop_back(token), None);
        });

        GhostToken::new(|ref mut token| {
            let arena = Arena::new();
            let mut list = LinkedList::new(&arena);

            list.push_back(1, token);
            list.push_back(2, token);

            assert_eq!(list.pop_front(token), Some(1));
            assert_eq!(list.pop_front(token), Some(2));
            assert_eq!(list.pop_front(token), None);
        });

        GhostToken::new(|ref mut token| {
            let arena = Arena::new();
            let mut list = LinkedList::new(&arena);

            list.push_front(1, token);
            list.push_front(2, token);

            assert_eq!(list.pop_back(token), Some(1));
            assert_eq!(list.pop_back(token), Some(2));
            assert_eq!(list.pop_back(token), None);
        });
    }

    #[test]
    pub fn iter() {
        GhostToken::new(|ref mut token| {
            let arena = Arena::new();
            let mut list = LinkedList::new(&arena);

            list.push_back(1, token);
            list.push_back(2, token);
            list.push_back(3, token);
            list.push_back(4, token);

            assert_eq!(
                list.iter(token).copied().collect::<Vec<_>>(),
                vec![1, 2, 3, 4]
            )
        });
    }
}
