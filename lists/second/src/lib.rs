#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
/// A doubly linked list using `StaticRc` and `GhostCell`
///
/// Allocation size per value:
/// sizeof: GhostCell<Node<T>> = max(usize, T) + usize + usize
/// => Overhead of list is between 16 and 24 bytes per entry
/// 
/// Pros:
/// - Fully `no_std`
/// - `StaticRc` and `GhostCell` are (almost) transparent types (`StaticRc` allocates) and have equivalent perf to raw pointers.
/// - `StaticRc` ensures **at compile time** that no pointers to removed nodes can't exist (list always stays valid)
/// - iterators and cursors work
/// - Insert and remove data without copying
/// - No re-allocation required
/// - Implementation is `Send` and `Sync`
///
/// Cons:
/// - requires tokens to be passed around
/// - mutable cursors and iterators are unsafe
/// - Allocates every node on the heap individually
/// - requires nightly rust
/// - list must be cleared before drop (will panic otherwise)
///
use ghost_cell::{GhostCell, GhostToken};
use static_rc::StaticRc;

pub struct LinkedList<'id, T> {
    len: usize,
    head_tail: Option<(HalfNodePtr<'id, T>, HalfNodePtr<'id, T>)>,
}

impl<'id, T> LinkedList<'id, T> {
    pub fn new() -> Self {
        Self {
            head_tail: None,
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn iter<'a>(&'a self, token: &'a GhostToken<'id>) -> Iter<'a, 'id, T> {
        Iter::new(token, self)
    }

    pub fn push_front(&mut self, value: T, token: &mut GhostToken<'id>) {
        let (one, two) = Self::new_halves(value);

        let head_tail = if let Some((head, tail)) = self.head_tail.take() {
            head.borrow_mut(token).prev = Some(one);
            two.borrow_mut(token).next = Some(head);

            (two, tail)
        } else {
            (one, two)
        };

        self.head_tail = Some(head_tail);
        self.len += 1;
    }

    pub fn push_back(&mut self, value: T, token: &mut GhostToken<'id>) {
        let (one, two) = Self::new_halves(value);

        let head_tail = if let Some((head, tail)) = self.head_tail.take() {
            tail.borrow_mut(token).next = Some(one);
            two.borrow_mut(token).prev = Some(tail);

            (head, two)
        } else {
            (one, two)
        };

        self.head_tail = Some(head_tail);
        self.len += 1;
    }

    pub fn pop_front(&mut self, token: &mut GhostToken<'id>) -> Option<T> {
        let (head, tail) = self.head_tail.take()?;

        if StaticRc::ptr_eq(&head, &tail) {
            return Some(Self::into_inner(head, tail));
        }

        let new_head = head.borrow_mut(token).next.take().unwrap();
        let other_head = new_head.borrow_mut(token).prev.take().unwrap();

        self.head_tail = Some((new_head, tail));
        self.len -= 1;

        Some(Self::into_inner(head, other_head))
    }

    pub fn pop_back(&mut self, token: &mut GhostToken<'id>) -> Option<T> {
        let (head, tail) = self.head_tail.take()?;

        if StaticRc::ptr_eq(&head, &tail) {
            return Some(Self::into_inner(head, tail));
        }

        let new_tail = tail.borrow_mut(token).prev.take().unwrap();
        let other_tail = new_tail.borrow_mut(token).next.take().unwrap();

        self.head_tail = Some((head, new_tail));
        self.len -= 1;

        Some(Self::into_inner(tail, other_tail))
    }

    pub fn clear(&mut self, token: &mut GhostToken<'id>) {
        while self.pop_front(token).is_some() {}
    }

    fn new_halves(value: T) -> (HalfNodePtr<'id, T>, HalfNodePtr<'id, T>) {
        let node = GhostCell::new(Node {
            value,
            prev: None,
            next: None,
        });

        let full = FullNodePtr::new(node);

        FullNodePtr::split::<1, 1>(full)
    }

    fn into_inner(left: HalfNodePtr<'id, T>, right: HalfNodePtr<'id, T>) -> T {
        let full = FullNodePtr::join(left, right);
        let ghost_cell = FullNodePtr::into_inner(full);

        let node = GhostCell::into_inner(ghost_cell);

        debug_assert!(node.prev.is_none());
        debug_assert!(node.next.is_none());

        node.value
    }
}

pub struct Iter<'a, 'id, T> {
    token: &'a GhostToken<'id>,
    head_tail: Option<(&'a GhostNode<'id, T>, &'a GhostNode<'id, T>)>,
}

impl<'a, 'id, T> Iter<'a, 'id, T> {
    pub fn new(token: &'a GhostToken<'id>, list: &'a LinkedList<'id, T>) -> Self {
        let head_tail = list
            .head_tail
            .as_ref()
            .map(|head_tail| (&*head_tail.0, &*head_tail.1));

        Self { token, head_tail }
    }
}

impl<'a, 'id, T> Iterator for Iter<'a, 'id, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let (head, tail) = self.head_tail.take()?;

        let current = head.borrow(self.token);

        if head as *const _ != tail as *const _ {
            self.head_tail = current.next.as_ref().map(|n| {
                let n: &'a GhostNode<'_, _> = &*n;
                (n, tail)
            });
        } else {
            self.head_tail = None;
        }

        Some(&current.value)
    }
}

impl<'a, 'id, T> DoubleEndedIterator for Iter<'a, 'id, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (head, tail) = self.head_tail.take()?;

        let node = tail.borrow(self.token);

        if head as *const _ != tail as *const _ {
            self.head_tail = node.prev.as_ref().map(|n| {
                let n: &'a GhostNode<'_, _> = &*n;
                (head, n)
            });
        } else {
            self.head_tail = None;
        }

        Some(&node.value)
    }
}

pub struct Node<'id, T> {
    value: T,
    prev: Option<HalfNodePtr<'id, T>>,
    next: Option<HalfNodePtr<'id, T>>,
}

pub type GhostNode<'id, T> = GhostCell<'id, Node<'id, T>>;

pub type HalfNodePtr<'id, T> = StaticRc<GhostNode<'id, T>, 1, 2>;
pub type FullNodePtr<'id, T> = StaticRc<GhostNode<'id, T>, 2, 2>;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn push_pop() {
        GhostToken::new(|ref mut token| {
            let mut list = LinkedList::new();

            list.push_front(1, token);
            list.push_front(2, token);

            assert_eq!(list.pop_front(token), Some(2));
            assert_eq!(list.pop_front(token), Some(1));
            assert_eq!(list.pop_front(token), None);

            list.clear(token)
        });

        GhostToken::new(|ref mut token| {
            let mut list = LinkedList::new();

            list.push_back(1, token);
            list.push_back(2, token);

            assert_eq!(list.pop_back(token), Some(2));
            assert_eq!(list.pop_back(token), Some(1));
            assert_eq!(list.pop_back(token), None);

            list.clear(token)
        });

        GhostToken::new(|ref mut token| {
            let mut list = LinkedList::new();

            list.push_back(1, token);
            list.push_back(2, token);

            assert_eq!(list.pop_front(token), Some(1));
            assert_eq!(list.pop_front(token), Some(2));
            assert_eq!(list.pop_front(token), None);

            list.clear(token)
        });

        GhostToken::new(|ref mut token| {
            let mut list = LinkedList::new();

            list.push_front(1, token);
            list.push_front(2, token);

            assert_eq!(list.pop_back(token), Some(1));
            assert_eq!(list.pop_back(token), Some(2));
            assert_eq!(list.pop_back(token), None);

            list.clear(token)
        });
    }

    #[test]
    fn send_sync() {
        GhostToken::new(|ref mut token| {
            let mut list = LinkedList::new();

            std::thread::scope(|s| {
                s.spawn(|| {
                    list.push_back(1, token);
                });
            });

            let len = std::thread::scope(|s| s.spawn(|| list.len()).join().unwrap());

            assert_eq!(len, 1);

            list.clear(token)
        })
    }

    #[derive(Default)]
    struct Big([usize; 32]);

    #[test]
    fn push_back_first_big() {
        GhostToken::new(|ref mut token| {
            let mut list = LinkedList::new();

            for _ in 0..500 {
                list.push_back(Big::default(), token);
            }
        });
    }

    #[test]
    fn node_size() {
        // sizeof: GhostCell<T> = T

        // Node<T> {
        // value: T
        // prev: usize
        // next: usize
        // }

        // sizeof: GhostCell<Node<T>> = max(usize, T) + usize + usize
        // 272 = 256 + 8 + 8

        panic!("{}", std::mem::size_of::<GhostCell<'_, Node<Big>>>());
    }
}
