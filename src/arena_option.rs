use ghost_cell::{GhostCell, GhostCursor, GhostToken};
use typed_arena::Arena;

pub struct LinkedList<'arena, 'id, T> {
    arena: &'arena Arena<Option<Node<'arena, 'id, T>>>,
    head_tail: Option<(NodeRef<'arena, 'id, T>, NodeRef<'arena, 'id, T>)>,
}

impl<'arena, 'id, T> LinkedList<'arena, 'id, T> {
    pub fn new(arena: &'arena Arena<Option<Node<'arena, 'id, T>>>) -> Self {
        Self {
            arena,
            head_tail: None,
        }
    }

    pub fn from_iter<I: IntoIterator<Item = T>>(
        iter: I,
        arena: &'arena Arena<Option<Node<'arena, 'id, T>>>,
        token: &mut GhostToken<'id>,
    ) -> Self {
        let mut list = LinkedList::new(arena);

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

    pub fn iter<'a>(&'a self, token: &'a GhostToken<'id>) -> Iter<'a, 'arena, 'id, T> {
        Iter {
            inner: Cursor::new_front(self, token),
        }
    }

    pub fn cursor_front<'a>(&'a self, token: &'a GhostToken<'id>) -> Cursor<'a, 'arena, 'id, T> {
        Cursor::new_front(self, token)
    }

    pub fn cursor_back<'a>(&'a self, token: &'a GhostToken<'id>) -> Cursor<'a, 'arena, 'id, T> {
        Cursor::new_back(self, token)
    }

    pub fn cursor_front_mut<'a>(
        &'a mut self,
        token: &'a mut GhostToken<'id>,
    ) -> CursorMut<'a, 'arena, 'id, T> {
        CursorMut::new_front(self, token)
    }

    pub fn cursor_back_mut<'a>(
        &'a mut self,
        token: &'a mut GhostToken<'id>,
    ) -> CursorMut<'a, 'arena, 'id, T> {
        CursorMut::new_front(self, token)
    }

    pub fn push_front(&mut self, value: T, token: &mut GhostToken<'id>) {
        let new_node = Node::new(value, self.arena);

        let head_tail = if let Some((head, tail)) = self.head_tail.take() {
            head.borrow_mut(token).as_mut().unwrap().left = Some(new_node);
            new_node.borrow_mut(token).as_mut().unwrap().right = Some(head);

            (new_node, tail)
        } else {
            (new_node, new_node)
        };

        self.head_tail = Some(head_tail)
    }

    pub fn push_back(&mut self, value: T, token: &mut GhostToken<'id>) {
        let new_node = Node::new(value, self.arena);

        let head_tail = if let Some((head, tail)) = self.head_tail.take() {
            tail.borrow_mut(token).as_mut().unwrap().right = Some(new_node);
            new_node.borrow_mut(token).as_mut().unwrap().left = Some(tail);

            (head, new_node)
        } else {
            (new_node, new_node)
        };

        self.head_tail = Some(head_tail)
    }

    pub fn pop_front(&mut self, token: &mut GhostToken<'id>) -> Option<T> {
        let (head, tail) = self.head_tail.take()?;

        // when there is only one element in the list
        if head.as_ptr() == tail.as_ptr() {
            return head.take(token).map(|node| node.value);
        }

        let next = head
            .borrow_mut(token)
            .as_mut()
            .unwrap()
            .right
            .take()
            .expect("Non-tail should have a right node");

        let _ = next
            .borrow_mut(token)
            .as_mut()
            .unwrap()
            .left
            .take()
            .expect("Non-head should have a left node");

        self.head_tail = Some((next, tail));

        head.take(token).map(|node| node.value)
    }

    pub fn pop_back(&mut self, token: &mut GhostToken<'id>) -> Option<T> {
        let (head, tail) = self.head_tail.take()?;

        // when there is only one element in the list
        if head.as_ptr() == tail.as_ptr() {
            return head.take(token).map(|node| node.value);
        }

        let prev = tail
            .borrow_mut(token)
            .as_mut()
            .unwrap()
            .left
            .take()
            .expect("Non-head should have a left node");
        let _ = prev
            .borrow_mut(token)
            .as_mut()
            .unwrap()
            .right
            .take()
            .expect("Non-tail should have a right node");

        self.head_tail = Some((head, prev));

        head.take(token).map(|node| node.value)
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

            mid_tail.borrow_mut(token).as_mut().unwrap().right = Some(mid_head);

            let right = static_rc::lift_with_mut(Some(mid_tail), token, |mid_tail, token| {
                let mut cursor = GhostCursor::new(token, Some(mid_tail.as_ref().unwrap()));

                cursor
                    .move_mut(|mid_tail| {
                        mid_tail
                            .as_ref()
                            .unwrap()
                            .right
                            .as_ref()
                            .map(core::borrow::Borrow::borrow)
                    })
                    .expect("mid_tail.next was just set!");

                let mid_head = cursor.into_inner().expect("mid_head was just computed!");

                &mut mid_head.as_mut().unwrap().left
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
        let mut head = Self::new(self.arena);

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

    // fn new_halves(&self, value: T) -> (HalfNodePtr<'arena, 'id, T>, HalfNodePtr<'arena, 'id, T>) {
    //     let full = FullNodePtr::new(Node::new(value, self.arena));

    //     StaticRcRef::split::<1, 1>(full)
    // }

    // fn into_inner(
    //     left: HalfNodePtr<'arena, 'id, T>,
    //     right: HalfNodePtr<'arena, 'id, T>,
    //     token: &mut GhostToken<'id>,
    // ) -> T {
    //     let full = FullNodePtr::join(left, right);
    //     let ghost_cell = FullNodePtr::into_inner(full);
    //     let node = ghost_cell.take(token).unwrap();

    //     //  If the node still has a prev and next, they are leaked.
    //     debug_assert!(node.left.is_none());
    //     debug_assert!(node.right.is_none());

    //     node.value
    // }

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

type NodeRef<'arena, 'id, T> = &'arena GhostCell<'id, Option<Node<'arena, 'id, T>>>;

pub struct Node<'arena, 'id, T> {
    value: T,
    left: Option<NodeRef<'arena, 'id, T>>,
    right: Option<NodeRef<'arena, 'id, T>>,
}

impl<'arena, 'id, T> Node<'arena, 'id, T> {
    fn new(
        value: T,
        arena: &'arena Arena<Option<Node<'arena, 'id, T>>>,
    ) -> NodeRef<'arena, 'id, T> {
        GhostCell::from_mut(arena.alloc(Some(Self {
            value,
            left: None,
            right: None,
        })))
    }
}

pub struct Cursor<'a, 'arena, 'id, T> {
    token: &'a GhostToken<'id>,
    node: Option<NodeRef<'arena, 'id, T>>,
}

impl<'a, 'arena, 'id, T> Cursor<'a, 'arena, 'id, T>
where
    'arena: 'a,
{
    pub fn new_front(list: &'a LinkedList<'arena, 'id, T>, token: &'a GhostToken<'id>) -> Self {
        let node = list.head_tail.as_ref().map(|head_tail| &*head_tail.0);

        Self { token, node }
    }

    pub fn new_back(list: &'a LinkedList<'arena, 'id, T>, token: &'a GhostToken<'id>) -> Self {
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
        self.node?
            .borrow(self.token)
            .as_ref()
            .map(|node| &node.value)
    }

    pub fn peek_right(&self) -> Option<&'a T> {
        self.peek_right_node()?
            .borrow(self.token)
            .as_ref()
            .map(|node| &node.value)
    }

    pub fn peek_left(&self) -> Option<&'a T> {
        self.peek_left_node()?
            .borrow(self.token)
            .as_ref()
            .map(|node| &node.value)
    }

    fn peek_right_node(&self) -> Option<NodeRef<'arena, 'id, T>> {
        self.node?.borrow(self.token).as_ref()?.right
    }

    fn peek_left_node(&self) -> Option<NodeRef<'arena, 'id, T>>
    {
        self.node?.borrow(self.token).as_ref()?.left
    }
}

pub struct CursorMut<'a, 'arena, 'id, T> {
    inner: GhostCursor<'a, 'id, Option<Node<'arena, 'id, T>>>,
}

impl<'a, 'arena, 'id, T> CursorMut<'a, 'arena, 'id, T>
where
    'arena: 'a,
{
    pub fn new_front(list: &'a LinkedList<'arena, 'id, T>, token: &'a mut GhostToken<'id>) -> Self {
        let node = list.head_tail.as_ref().map(|head_tail| &*head_tail.0);

        Self {
            inner: GhostCursor::new(token, node),
        }
    }

    pub fn new_back(list: &'a LinkedList<'arena, 'id, T>, token: &'a mut GhostToken<'id>) -> Self {
        let node = list.head_tail.as_ref().map(|head_tail| &*head_tail.1);

        Self {
            inner: GhostCursor::new(token, node),
        }
    }

    pub fn into_cursor(self) -> Cursor<'a, 'arena, 'id, T>
    where
        'a: 'arena,
    {
        let (token, node) = self.inner.into_parts();

        Cursor { token, node }
    }

    pub fn move_right(&mut self) -> bool {
        self.inner.move_mut(|node| node.as_ref()?.right).is_ok()
    }

    pub fn move_left(&mut self) -> bool {
        self.inner.move_mut(|node| node.as_ref()?.left).is_ok()
    }

    pub fn current(&mut self) -> Option<&mut T> {
        self.inner
            .borrow_mut()?
            .as_mut()
            .map(|node| &mut node.value)
    }

    pub fn peek_right(&self) -> Option<&T> {
        let token = self.inner.token();

        self.peek_right_node()?
            .borrow(token)
            .as_ref()
            .map(|node| &node.value)
    }

    pub fn peek_left(&self) -> Option<&T> {
        let token = self.inner.token();

        self.peek_left_node()?
            .borrow(token)
            .as_ref()
            .map(|node| &node.value)
    }

    fn peek_right_node(&self) -> Option<NodeRef<'arena, 'id, T>> {
        self.inner.borrow()?.as_ref()?.right
    }

    fn peek_left_node(&self) -> Option<NodeRef<'arena, 'id, T>> {
        self.inner.borrow()?.as_ref()?.left
    }
}

pub struct Iter<'a, 'arena, 'id, T> {
    inner: Cursor<'a, 'arena, 'id, T>,
}

impl<'a, 'arena, 'id, T> Iterator for Iter<'a, 'arena, 'id, T>
where
    T: 'a,
    'arena: 'a,
{
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
            let arena = Arena::new();
            let mut list = LinkedList::from_iter([1, 2, 3], &arena, token);
            let mut other = LinkedList::from_iter([4, 5, 6], &arena, token);

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
            drop(iter);

            list.clear(token);
            other.clear(token);
        })
    }

    #[test]
    fn prepend() {
        GhostToken::new(|ref mut token| {
            let arena = Arena::new();
            let mut list = LinkedList::from_iter([1, 2, 3], &arena, token);
            let mut other = LinkedList::from_iter([4, 5, 6], &arena, token);

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
            let arena = Arena::new();
            let mut list =
                LinkedList::from_iter([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], &arena, token);

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
