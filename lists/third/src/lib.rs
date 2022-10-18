#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]

/// A doubly linked list using `slotmap` arena allocation
///
/// Allocation size per value:
/// sizeof: Node<T> = max(4, T) + usize + usize
/// => Overhead of list is between 16 and 19 bytes per entry
/// 
/// Pros:
/// - *Fast* - Arena allocation is wayy faster than traditional heap allocation, freeing is even faster
/// - Fully `no_std`
/// - No unsafe & fully stable rust
/// - Implementation is `Send` and `Sync`
/// - No tokens required
/// - Iteration works
///
/// Cons:
/// - lot's of `unwrap()` => no compile time guarantees (similar to first's explicit `drop`)
/// - still 3x slower than `VecDeque`
use slotmap::{DefaultKey, SlotMap};

pub struct LinkedList<T> {
    len: usize,
    nodes: SlotMap<DefaultKey, Node<T>>,
    head_tail: Option<(DefaultKey, DefaultKey)>,
}

impl<T> LinkedList<T> {
    pub fn new() -> Self {
        Self {
            head_tail: None,
            nodes: SlotMap::new(),
            len: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            head_tail: None,
            nodes: SlotMap::with_capacity(capacity),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn iter<'a>(&'a self) -> Iter<'a, T> {
        Iter {
            list: self,
            head_tail: self.head_tail,
        }
    }

    pub fn push_front(&mut self, value: T) {
        let new_head = self.insert(value);

        let head_tail = if let Some((head, tail)) = self.head_tail.take() {
            self.get_mut(head).unwrap().prev = Some(new_head);
            self.get_mut(new_head).unwrap().next = Some(head);

            (new_head, tail)
        } else {
            (new_head, new_head)
        };

        self.head_tail = Some(head_tail);
        self.len += 1;
    }

    pub fn push_back(&mut self, value: T) {
        let new_tail = self.insert(value);

        let head_tail = if let Some((head, tail)) = self.head_tail.take() {
            self.get_mut(tail).unwrap().next = Some(new_tail);
            self.get_mut(new_tail).unwrap().prev = Some(tail);

            (head, new_tail)
        } else {
            (new_tail, new_tail)
        };

        self.head_tail = Some(head_tail);
        self.len += 1;
    }

    pub fn pop_front(&mut self) -> Option<T> {
        let (head, tail) = self.head_tail.take()?;

        if head == tail {
            return Some(self.remove(head).unwrap().value);
        }

        let new_head = self.get_mut(head).unwrap().next.take().unwrap();
        self.get_mut(new_head).unwrap().prev.take().unwrap();

        self.head_tail = Some((new_head, tail));
        self.len -= 1;

        Some(self.remove(head).unwrap().value)
    }

    pub fn pop_back(&mut self) -> Option<T> {
        let (head, tail) = self.head_tail.take()?;

        if head == tail {
            // they are pointing to the same thing, but let's keep the symmetry
            Some(self.remove(tail).unwrap().value);
        }

        let new_tail = self.get_mut(head).unwrap().prev.take().unwrap();
        self.get_mut(new_tail).unwrap().next.take().unwrap();

        self.head_tail = Some((head, new_tail));
        self.len -= 1;

        Some(self.remove(tail).unwrap().value)
    }

    pub fn clear(&mut self) {
        while self.pop_front().is_some() {}
    }

    fn insert(&mut self, value: T) -> DefaultKey {
        self.nodes.insert(Node {
            value,
            prev: None,
            next: None,
        })
    }

    fn get_mut(&mut self, node_ref: DefaultKey) -> Option<&mut Node<T>> {
        self.nodes.get_mut(node_ref)
    }

    fn get(&self, node_ref: DefaultKey) -> Option<&Node<T>> {
        self.nodes.get(node_ref)
    }

    fn remove(&mut self, node_ref: DefaultKey) -> Option<Node<T>> {
        self.nodes.remove(node_ref)
    }
}

struct Node<T> {
    value: T,
    prev: Option<DefaultKey>,
    next: Option<DefaultKey>,
}

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

pub struct Iter<'a, T> {
    list: &'a LinkedList<T>,
    head_tail: Option<(DefaultKey, DefaultKey)>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let (head, tail) = self.head_tail.take()?;

        let node = self.list.get(head).unwrap();

        if head != tail {
            self.head_tail = node.next.map(|n| (n, tail));
        } else {
            self.head_tail = None;
        }

        Some(&node.value)
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (head, tail) = self.head_tail.take()?;

        let node = self.list.get(tail).unwrap();

        if head != tail {
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

    #[test]
    pub fn iter() {
        let mut list = LinkedList::new();

        list.push_back(1);
        list.push_back(2);
        list.push_back(3);
        list.push_back(4);

        assert_eq!(list.into_iter().collect::<Vec<_>>(), vec![1, 2, 3, 4])
    }

    #[test]
    fn send_sync() {
        let mut list = LinkedList::new();

        std::thread::scope(|s| {
            s.spawn(|| {
                list.push_back(1);
            });
        });

        let len = std::thread::scope(|s| s.spawn(|| list.len()).join().unwrap());

        assert_eq!(len, 1);
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

    #[test]
    fn node_size() {
        // sizeof: DefaultKey = 8

        // sizeof: Node<u8> = 20 (- 1 = 19)
        // sizeof: Node<u16> = 20 (- 2 = 18)
        // sizeof: Node<u32> = 20 (- 4 = 16)
        // sizeof: Node<usize> = 24 (- 8 = 16)
        // sizeof: Node<Big> = 272 (- 8 = 16)

        // sizeof: Node<T> = max(4, T) + usize + usize

        panic!("{}", std::mem::size_of::<Node<Big>>());
    }
}
