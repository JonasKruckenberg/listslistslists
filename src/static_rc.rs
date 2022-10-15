use ghost_cell::{GhostCell, GhostCursor, GhostToken};
use static_rc::StaticRc;

pub struct LinkedList<'id, T> {
    head_tail: Option<(HalfNodePtr<'id, T>, HalfNodePtr<'id, T>)>,
}

impl<'id, T> LinkedList<'id, T> {
    pub fn new() -> Self {
        Self { head_tail: None }
    }

    pub fn from_iter<I: IntoIterator<Item = T>>(iter: I, token: &mut GhostToken<'id>) -> Self {
        let mut list = LinkedList::new();

        for value in iter.into_iter() {
            list.push_back(value, token)
        }

        list
    }

    pub fn len(&self, token: &GhostToken<'id>) -> usize {
        self.iter(token).count()
    }

    pub fn is_empty(&self) -> bool {
        self.head_tail.is_none()
    }

    pub fn iter<'a>(&'a self, token: &'a GhostToken<'id>) -> Iter<'a, 'id, T> {
        Iter {
            inner: Cursor::new_front(self, token),
        }
    }

    pub fn cursor_front<'a>(&'a self, token: &'a GhostToken<'id>) -> Cursor<'a, 'id, T> {
        Cursor::new_front(self, token)
    }

    pub fn cursor_back<'a>(&'a self, token: &'a GhostToken<'id>) -> Cursor<'a, 'id, T> {
        Cursor::new_back(self, token)
    }

    pub fn cursor_front_mut<'a>(
        &'a mut self,
        token: &'a mut GhostToken<'id>,
    ) -> CursorMut<'a, 'id, T> {
        CursorMut::new_front(self, token)
    }

    pub fn cursor_back_mut<'a>(
        &'a mut self,
        token: &'a mut GhostToken<'id>,
    ) -> CursorMut<'a, 'id, T> {
        CursorMut::new_front(self, token)
    }

    pub fn push_front(&mut self, value: T, token: &mut GhostToken<'id>) {
        let (one, two) = Self::new_halves(value);

        let head_tail = if let Some((head, tail)) = self.head_tail.take() {
            head.borrow_mut(token).left = Some(one);
            two.borrow_mut(token).right = Some(head);

            (two, tail)
        } else {
            (one, two)
        };

        self.head_tail = Some(head_tail)
    }

    pub fn push_back(&mut self, value: T, token: &mut GhostToken<'id>) {
        let (one, two) = Self::new_halves(value);

        let head_tail = if let Some((head, tail)) = self.head_tail.take() {
            tail.borrow_mut(token).right = Some(one);
            two.borrow_mut(token).left = Some(tail);

            (head, two)
        } else {
            (one, two)
        };

        self.head_tail = Some(head_tail)
    }

    pub fn pop_front(&mut self, token: &mut GhostToken<'id>) -> Option<T> {
        let (head, tail) = self.head_tail.take()?;

        // when there is only one element in the list
        if StaticRc::as_ptr(&head) == StaticRc::as_ptr(&tail) {
            return Some(Self::into_inner(head, tail));
        }

        let next = head.borrow_mut(token).right.take().unwrap();
        let other_head = next.borrow_mut(token).left.take().unwrap();

        self.head_tail = Some((next, tail));

        Some(Self::into_inner(head, other_head))
    }

    pub fn pop_back(&mut self, token: &mut GhostToken<'id>) -> Option<T> {
        let (head, tail) = self.head_tail.take()?;

        // when there is only one element in the list
        if StaticRc::as_ptr(&head) == StaticRc::as_ptr(&tail) {
            return Some(Self::into_inner(head, tail));
        }

        let prev = tail
            .borrow_mut(token)
            .left
            .take()
            .expect("Non-head should have a left node");
        let other_tail = prev
            .borrow_mut(token)
            .right
            .take()
            .expect("Non-tail should have a right node");

        self.head_tail = Some((head, prev));

        Some(Self::into_inner(tail, other_tail))
    }

    pub fn append(&mut self, other: &mut Self, token: &mut GhostToken<'id>) {
        let other_ht = if let Some(other_ht) = other.head_tail.take() {
            other_ht
        } else {
            return;
        };

        if let Some(self_ht) = self.head_tail.take() {
            let (head, mid_tail) = self_ht;
            let (mid_head, tail) = other_ht;

            mid_tail.borrow_mut(token).right = Some(mid_head);

            let right = static_rc::lift_with_mut(Some(mid_tail), token, |mid_tail, token| {
                let mut cursor = GhostCursor::new(token, Some(mid_tail.as_ref().unwrap()));

                cursor
                    .move_mut(|mid_tail| mid_tail.right.as_ref().map(core::borrow::Borrow::borrow))
                    .expect("mid_tail.next was just set!");

                let mid_head = cursor.into_inner().expect("mid_head was just computed!");

                &mut mid_head.left
            });

            debug_assert!(
                right.is_none(),
                "mid_head should not have had any previous!"
            );

            self.head_tail = Some((head, tail));
        } else {
            self.head_tail = Some(other_ht)
        }
    }

    pub fn prepend(&mut self, other: &mut Self, token: &mut GhostToken<'id>) {
        other.append(self, token);
        core::mem::swap(self, other);
    }

    // pub fn split_off(&mut self, at: usize, token: &mut GhostToken<'id>) -> Option<Self> {
    // let mut cursor = GhostCursor::new(
    //     token,
    //     self.head_tail
    //         .as_ref()
    //         .map(|(head, _)| core::borrow::Borrow::borrow(head)),
    // );

    //     for _ in 0..at {
    //         cursor
    //             .move_mut(|node| node.right.as_ref().map(core::borrow::Borrow::borrow))
    //             .ok()?;
    //     }

    //     let mid_head = cursor.borrow_mut()?.right.take()?;

    //     cursor
    //         .move_mut(|node| node.right.as_ref().map(core::borrow::Borrow::borrow))
    //         .ok()?;

    //     let mid_tail = cursor.borrow_mut()?.left.take()?;

    //     let (head, tail) = self.head_tail.take()?;
    //     self.head_tail = Some((head, mid_tail));

    //     let mut other = DLList::new();
    //     other.head_tail = Some((mid_head, tail));

    //     Some(other)
    // }

    pub fn split_off(&mut self, at: usize, token: &mut GhostToken<'id>) -> Option<Self> {
        //  This is not the most optimal implementation, but it works, and respects the promised algorithmic complexity.
        let mut head = Self::new();

        for _ in 0..at {
            let element = if let Some(element) = self.pop_front(token) {
                element
            } else {
                //  Restore popped elements, and pretend that nothing happened.
                self.prepend(&mut head, token);
                return None;
            };
            head.push_back(element, token);
        }

        core::mem::swap(self, &mut head);

        Some(head)
    }

    pub fn clear(&mut self, token: &mut GhostToken<'id>) {
        while self.pop_back(token).is_some() {}
    }

    fn new_halves(value: T) -> (HalfNodePtr<'id, T>, HalfNodePtr<'id, T>) {
        let node = Node {
            value,
            left: None,
            right: None,
        };
        let full = FullNodePtr::new(GhostNode::new(node));

        StaticRc::split::<1, 1>(full)
    }

    fn into_inner(left: HalfNodePtr<'id, T>, right: HalfNodePtr<'id, T>) -> T {
        let full = FullNodePtr::join(left, right);
        let ghost_cell = FullNodePtr::into_inner(full);
        let node = GhostNode::into_inner(ghost_cell);

        //  If the node still has a prev and next, they are leaked.
        debug_assert!(node.left.is_none());
        debug_assert!(node.right.is_none());

        node.value
    }

    // pub fn dot<W: core::fmt::Write>(&self, f: &mut W, token: &GhostToken<'id>) -> core::fmt::Result
    // where
    //     T: std::fmt::Display,
    // {
    //     writeln!(f, "digraph {{")?;
    //     writeln!(f, "rankdir=LR;")?;
    //     writeln!(f, "node [shape=record];")?;
    //     writeln!(f, "0 [label=\"nil\" shape=circle];")?;

    //     let mut i = 1;

    //     let mut c = Cursor::new_front(self, token);

    //     loop {
    //         let node = c.node.as_ref().unwrap().borrow(token);

    //         writeln!(
    //             f,
    //             "{} [label=\"{{ <left> | <data> {} | <right> }}\"];",
    //             i, node.value
    //         )?;

    //         writeln!(
    //             f,
    //             "{}:left:c -> {}:data:n [arrowhead=vee, arrowtail=dot, dir=both, tailclip=false];",
    //             i,
    //             i - 1
    //         )?;
    //         writeln!(
    //             f,
    //             "{}:right:c -> {}:data:s [arrowhead=vee, arrowtail=dot, dir=both, tailclip=false];",
    //             i,
    //             i + 1
    //         )?;

    //         i += 1;

    //         if !c.move_right() {
    //             break;
    //         }
    //     }

    //     writeln!(f, "{} [label=\"nil\" shape=circle];", i)?;
    //     write!(f, "}}")?;

    //     Ok(())
    // }
}

pub struct Node<'id, T> {
    value: T,
    left: Option<HalfNodePtr<'id, T>>,
    right: Option<HalfNodePtr<'id, T>>,
}

type GhostNode<'id, T> = GhostCell<'id, Node<'id, T>>;

type HalfNodePtr<'id, T> = StaticRc<GhostNode<'id, T>, 1, 2>;
type FullNodePtr<'id, T> = StaticRc<GhostNode<'id, T>, 2, 2>;

pub struct Cursor<'a, 'id, T> {
    token: &'a GhostToken<'id>,
    node: Option<&'a GhostNode<'id, T>>,
}

impl<'a, 'id, T> Cursor<'a, 'id, T> {
    pub fn new_front(list: &'a LinkedList<'id, T>, token: &'a GhostToken<'id>) -> Self {
        let node = list.head_tail.as_ref().map(|head_tail| &*head_tail.0);

        Self { token, node }
    }

    pub fn new_back(list: &'a LinkedList<'id, T>, token: &'a GhostToken<'id>) -> Self {
        let node = list.head_tail.as_ref().map(|head_tail| &*head_tail.1);

        Self { token, node }
    }

    pub fn move_right(&mut self) -> bool {
        if let Some(node) = self.peek_right_node() {
            self.node = Some(node);

            true
        } else {
            self.node = None;

            false
        }
    }

    pub fn move_left(&mut self) -> bool {
        if let Some(node) = self.peek_left_node() {
            self.node = Some(node);

            true
        } else {
            self.node = None;

            false
        }
    }

    pub fn current(&self) -> Option<&'a T> {
        self.node.map(|node| &node.borrow(self.token).value)
    }

    pub fn peek_right(&self) -> Option<&'a T> {
        self.peek_right_node()
            .map(|node| &node.borrow(self.token).value)
    }

    pub fn peek_left(&self) -> Option<&'a T> {
        self.peek_left_node()
            .map(|node| &node.borrow(self.token).value)
    }

    fn peek_right_node(&self) -> Option<&'a GhostNode<'id, T>> {
        self.node?.borrow(self.token).right.as_deref()
    }

    fn peek_left_node(&self) -> Option<&'a GhostNode<'id, T>> {
        self.node?.borrow(self.token).left.as_deref()
    }
}

pub struct CursorMut<'a, 'id, T> {
    inner: GhostCursor<'a, 'id, Node<'id, T>>,
}

impl<'a, 'id, T> CursorMut<'a, 'id, T> {
    pub fn new_front(list: &'a LinkedList<'id, T>, token: &'a mut GhostToken<'id>) -> Self {
        let node = list.head_tail.as_ref().map(|head_tail| &*head_tail.0);

        Self {
            inner: GhostCursor::new(token, node),
        }
    }

    pub fn new_back(list: &'a LinkedList<'id, T>, token: &'a mut GhostToken<'id>) -> Self {
        let node = list.head_tail.as_ref().map(|head_tail| &*head_tail.1);

        Self {
            inner: GhostCursor::new(token, node),
        }
    }

    pub fn into_cursor(self) -> Cursor<'a, 'id, T> {
        let (token, node) = self.inner.into_parts();

        Cursor { token, node }
    }

    pub fn move_right(&mut self) -> bool {
        self.inner.move_mut(|node| node.right.as_deref()).is_ok()
    }

    pub fn move_left(&mut self) -> bool {
        self.inner.move_mut(|node| node.left.as_deref()).is_ok()
    }

    pub fn current(&mut self) -> Option<&mut T> {
        self.inner.borrow_mut().map(|node| &mut node.value)
    }

    pub fn peek_right(&self) -> Option<&T> {
        let token = self.inner.token();

        self.peek_right_node().map(|node| &node.borrow(token).value)
    }

    pub fn peek_left(&self) -> Option<&T> {
        let token = self.inner.token();

        self.peek_left_node().map(|node| &node.borrow(token).value)
    }

    fn peek_right_node(&self) -> Option<&GhostNode<'id, T>> {
        self.inner.borrow().and_then(|node| node.right.as_deref())
    }

    fn peek_left_node(&self) -> Option<&GhostNode<'id, T>> {
        self.inner.borrow().and_then(|node| node.left.as_deref())
    }
}

pub struct Iter<'a, 'id, T> {
    inner: Cursor<'a, 'id, T>,
}

impl<'a, 'id, T> Iterator for Iter<'a, 'id, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.inner.current();

        if self.inner.move_right() {}

        item
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn append() {
        GhostToken::new(|ref mut token| {
            let mut list = LinkedList::from_iter([1, 2, 3], token);
            let mut other = LinkedList::from_iter([4, 5, 6], token);

            list.append(&mut other, token);

            assert_eq!(list.len(token), 6);
            assert!(other.is_empty());

            let mut iter = list.iter(token).copied();
            assert_eq!(iter.next(), Some(1));
            assert_eq!(iter.next(), Some(2));
            assert_eq!(iter.next(), Some(3));
            assert_eq!(iter.next(), Some(4));
            assert_eq!(iter.next(), Some(5));
            assert_eq!(iter.next(), Some(6));
            assert_eq!(iter.next(), None);

            list.clear(token);
            other.clear(token);
        })
    }

    #[test]
    fn prepend() {
        GhostToken::new(|ref mut token| {
            let mut list = LinkedList::from_iter([1, 2, 3], token);
            let mut other = LinkedList::from_iter([4, 5, 6], token);

            list.prepend(&mut other, token);

            assert_eq!(list.len(token), 6);
            assert!(other.is_empty());

            let mut iter = list.iter(token).copied();
            assert_eq!(iter.next(), Some(4));
            assert_eq!(iter.next(), Some(5));
            assert_eq!(iter.next(), Some(6));
            assert_eq!(iter.next(), Some(1));
            assert_eq!(iter.next(), Some(2));
            assert_eq!(iter.next(), Some(3));
            assert_eq!(iter.next(), None);

            list.clear(token);
            other.clear(token);
        })
    }

    #[test]
    fn split_off() {
        GhostToken::new(|ref mut token| {
            let mut list = LinkedList::from_iter([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], token);

            let mut other = list.split_off(5, token).unwrap();

            assert_eq!(list.len(token), 5);
            assert_eq!(other.len(token), 7);

            let mut iter = list.iter(token).copied();
            assert_eq!(iter.next(), Some(1));
            assert_eq!(iter.next(), Some(2));
            assert_eq!(iter.next(), Some(3));
            assert_eq!(iter.next(), Some(4));
            assert_eq!(iter.next(), Some(5));
            assert_eq!(iter.next(), None);

            let mut iter = other.iter(token).copied();
            assert_eq!(iter.next(), Some(6));
            assert_eq!(iter.next(), Some(7));
            assert_eq!(iter.next(), Some(8));
            assert_eq!(iter.next(), Some(9));
            assert_eq!(iter.next(), Some(10));
            assert_eq!(iter.next(), Some(11));
            assert_eq!(iter.next(), Some(12));
            assert_eq!(iter.next(), None);

            list.clear(token);
            other.clear(token);
        })
    }
}
