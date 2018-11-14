use super::*;

use std::marker::PhantomData;
use std::cell::RefMut;

/// Used to position an element within another element.
pub trait LayoutEngine<E>
    where E: Extension
{
    type ChildData: 'static;

    fn name() -> &'static str;
    fn style_properties<'a, F>(prop: F)
        where F: FnMut(StaticKey) + 'a;

    fn new_child_data() -> Self::ChildData;

    fn update_data(&mut self, _styles: &Styles<E>, _nc: &NodeChain<E>, _rule: &Rule<E>) -> DirtyFlags {
        DirtyFlags::empty()
    }

    fn update_child_data(&mut self, _styles: &Styles<E>, _nc: &NodeChain<E>, _rule: &Rule<E>, _data: &mut Self::ChildData) -> DirtyFlags {
        DirtyFlags::empty()
    }

    fn reset_unset_data(&mut self, _used_keys: &FnvHashSet<StaticKey>) -> DirtyFlags {
        DirtyFlags::empty()
    }
    fn reset_unset_child_data(&mut self, _used_keys: &FnvHashSet<StaticKey>, _data: &mut Self::ChildData) -> DirtyFlags {
        DirtyFlags::empty()
    }

    fn check_parent_flags(&mut self, _flags: DirtyFlags) -> DirtyFlags {
        DirtyFlags::empty()
    }
    fn check_child_flags(&mut self, _flags: DirtyFlags) -> DirtyFlags {
        DirtyFlags::empty()
    }

    fn start_layout(&mut self, _ext: &mut E::NodeData, current: Rect, _flags: DirtyFlags, _children: ChildAccess<Self, E>) -> Rect {
        current
    }
    fn do_layout(&mut self, _value: &NodeValue<E>, _ext: &mut E::NodeData, _data: &mut Self::ChildData, current: Rect, _flags: DirtyFlags) -> Rect {
        current
    }
    fn do_layout_end(&mut self, _value: &NodeValue<E>, _ext: &mut E::NodeData, _data: &mut Self::ChildData, current: Rect, _flags: DirtyFlags) -> Rect {
        current
    }
    fn finish_layout(&mut self, _ext: &mut E::NodeData, current: Rect, _flags: DirtyFlags, _children: ChildAccess<Self, E>) -> Rect {
        current
    }
}

pub struct ChildAccess<'a, L: LayoutEngine<E> + ?Sized, E: Extension + 'a> {
    _l: PhantomData<L>,
    nodes: &'a [Node<E>],
}

pub struct NodeAccess<'a, L: LayoutEngine<E> + ?Sized, E: Extension + 'a> {
    node: RefMut<'a, NodeInner<E>>,
    _l: PhantomData<L>,
}

impl <'a, L, E> NodeAccess<'a, L, E>
    where L: LayoutEngine<E>,
          E: Extension
{
    pub fn split(&mut self) -> (&mut NodeValue<E>, &mut L::ChildData) {
        let node: &mut _ = &mut *self.node;
        (
            &mut node.value,
            node.parent_data.downcast_mut::<L::ChildData>()
                .expect("Child has incorrect data")
        )
    }
}

impl <'a, L, E> ChildAccess<'a, L, E>
    where L: LayoutEngine<E>,
          E: Extension
{
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn get(&self, idx: usize) -> Option<(Rect, DirtyFlags, NodeAccess<L, E>)> {
        let n = self.nodes.get(idx)?;
        let nr = n.inner.borrow_mut();
        let draw_rect = nr.draw_rect;
        let flags = nr.dirty_flags;

        Some((draw_rect, flags, NodeAccess {
            node: nr,
            _l: PhantomData,
        }))
    }
}


pub(crate) trait BoxLayoutEngine<E>
    where E: Extension
{
    fn name(&self) -> &'static str;
    fn update_data(&mut self, styles: &Styles<E>, nc: &NodeChain<E>, rule: &Rule<E>) -> DirtyFlags;
    fn update_child_data(&mut self, styles: &Styles<E>, nc: &NodeChain<E>, rule: &Rule<E>, data: &mut Box<Any>) -> DirtyFlags;
    fn reset_unset_data(&mut self, used_keys: &FnvHashSet<StaticKey>) -> DirtyFlags;
    fn reset_unset_child_data(&mut self, used_keys: &FnvHashSet<StaticKey>, data: &mut Box<Any>) -> DirtyFlags;
    fn check_parent_flags(&mut self, flags: DirtyFlags) -> DirtyFlags;
    fn check_child_flags(&mut self, flags: DirtyFlags) -> DirtyFlags;

    fn start_layout(&mut self, _ext: &mut E::NodeData, current: Rect, flags: DirtyFlags, children: &[Node<E>]) -> Rect;
    fn do_layout(&mut self, value: &NodeValue<E>, _ext: &mut E::NodeData, data: &mut Box<Any>, current: Rect, flags: DirtyFlags) -> Rect;
    fn do_layout_end(&mut self, value: &NodeValue<E>, _ext: &mut E::NodeData, data: &mut Box<Any>, current: Rect, flags: DirtyFlags) -> Rect;
    fn finish_layout(&mut self, _ext: &mut E::NodeData, current: Rect, flags: DirtyFlags, children: &[Node<E>]) -> Rect;
}

impl <E, T> BoxLayoutEngine<E> for T
    where E: Extension,
        T: LayoutEngine<E>
{
    fn name(&self) -> &'static str {
        T::name()
    }
    fn update_data(&mut self, styles: &Styles<E>, nc: &NodeChain<E>, rule: &Rule<E>) -> DirtyFlags {
        LayoutEngine::update_data(self, styles, nc, rule)
    }

    fn update_child_data(&mut self, styles: &Styles<E>, nc: &NodeChain<E>, rule: &Rule<E>, data: &mut Box<Any>) -> DirtyFlags {
        if !data.is::<<Self as LayoutEngine<E>>::ChildData>() {
            *data = Box::new(Self::new_child_data());
        }
        let data = data.downcast_mut::<<Self as LayoutEngine<E>>::ChildData>().expect("Failed to access child data");
        LayoutEngine::update_child_data(self, styles, nc, rule, data)
    }

    fn reset_unset_data(&mut self, used_keys: &FnvHashSet<StaticKey>) -> DirtyFlags {
        LayoutEngine::reset_unset_data(self, used_keys)
    }
    fn reset_unset_child_data(&mut self, used_keys: &FnvHashSet<StaticKey>, data: &mut Box<Any>) -> DirtyFlags {
        if !data.is::<<Self as LayoutEngine<E>>::ChildData>() {
            *data = Box::new(Self::new_child_data());
        }
        let data = data.downcast_mut::<<Self as LayoutEngine<E>>::ChildData>().expect("Failed to access child data");
        LayoutEngine::reset_unset_child_data(self, used_keys, data)
    }

    fn check_parent_flags(&mut self, flags: DirtyFlags) -> DirtyFlags {
        LayoutEngine::check_parent_flags(self, flags)
    }
    fn check_child_flags(&mut self, flags: DirtyFlags) -> DirtyFlags {
        LayoutEngine::check_child_flags(self, flags)
    }

    fn start_layout(&mut self, ext: &mut E::NodeData, current: Rect, flags: DirtyFlags, children: &[Node<E>]) -> Rect {
        LayoutEngine::start_layout(self, ext, current, flags, ChildAccess{_l: PhantomData, nodes: children})
    }
    fn do_layout(&mut self, value: &NodeValue<E>, ext: &mut E::NodeData, data: &mut Box<Any>, current: Rect, flags: DirtyFlags) -> Rect {
        let data = data.downcast_mut::<<Self as LayoutEngine<E>>::ChildData>().expect("Failed to access child data");
        LayoutEngine::do_layout(self, value, ext, data, current, flags)
    }
    fn do_layout_end(&mut self, value: &NodeValue<E>, ext: &mut E::NodeData, data: &mut Box<Any>, current: Rect, flags: DirtyFlags) -> Rect {
        let data = data.downcast_mut::<<Self as LayoutEngine<E>>::ChildData>().expect("Failed to access child data");
        LayoutEngine::do_layout_end(self, value, ext, data, current, flags)
    }
    fn finish_layout(&mut self, ext: &mut E::NodeData, current: Rect, flags: DirtyFlags, children: &[Node<E>]) -> Rect {
        LayoutEngine::finish_layout(self, ext, current, flags, ChildAccess{_l: PhantomData, nodes: children})
    }
}

#[derive(Default)]
pub(crate) struct AbsoluteLayout {
}
#[derive(Default)]
pub(crate) struct AbsoluteLayoutChild {
    x: Option<i32>,
    y: Option<i32>,
    width: Option<i32>,
    height: Option<i32>,
}

pub static X: StaticKey = StaticKey("x");
pub static Y: StaticKey = StaticKey("y");
pub static WIDTH: StaticKey = StaticKey("width");
pub static HEIGHT: StaticKey = StaticKey("height");

impl <E> LayoutEngine<E> for AbsoluteLayout
    where E: Extension
{
    type ChildData = AbsoluteLayoutChild;

    fn name() -> &'static str { "absolute" }
    fn style_properties<'a, F>(mut prop: F)
        where F: FnMut(StaticKey) + 'a
    {
        prop(X);
        prop(Y);
        prop(WIDTH);
        prop(HEIGHT);
    }

    fn new_child_data() -> AbsoluteLayoutChild {
        AbsoluteLayoutChild::default()
    }

    fn update_data(&mut self, _styles: &Styles<E>, _nc: &NodeChain<E>, _rule: &Rule<E>) -> DirtyFlags {
        DirtyFlags::empty()
    }
    fn update_child_data(&mut self, styles: &Styles<E>, nc: &NodeChain<E>, rule: &Rule<E>, data: &mut Self::ChildData) -> DirtyFlags {
        let mut flags = DirtyFlags::empty();
        eval!(styles, nc, rule.X => val => {
            let new = val.convert();
            if data.x != new {
                data.x = new;
                flags |= DirtyFlags::POSITION;
            }
        });
        eval!(styles, nc, rule.Y => val => {
            let new = val.convert();
            if data.y != new {
                data.y = new;
                flags |= DirtyFlags::POSITION;
            }
        });
        eval!(styles, nc, rule.WIDTH => val => {
            let new = val.convert();
            if data.width != new {
                data.width = new;
                flags |= DirtyFlags::SIZE;
            }
        });
        eval!(styles, nc, rule.HEIGHT => val => {
            let new = val.convert();
            if data.height != new {
                data.height = new;
                flags |= DirtyFlags::SIZE;
            }
        });
        flags
    }

    fn reset_unset_data(&mut self, _used_keys: &FnvHashSet<StaticKey>) -> DirtyFlags {
        DirtyFlags::empty()
    }
    fn reset_unset_child_data(&mut self, used_keys: &FnvHashSet<StaticKey>, data: &mut Self::ChildData) -> DirtyFlags {
        let mut flags = DirtyFlags::empty();
        if !used_keys.contains(&X) && data.x.is_some() {
            data.x = None;
            flags |= DirtyFlags::POSITION;
        }
        if !used_keys.contains(&Y) && data.y.is_some() {
            data.y = None;
            flags |= DirtyFlags::POSITION;
        }
        if !used_keys.contains(&WIDTH) && data.width.is_some() {
            data.width = None;
            flags |= DirtyFlags::SIZE;
        }
        if !used_keys.contains(&HEIGHT) && data.height.is_some() {
            data.height = None;
            flags |= DirtyFlags::SIZE;
        }

        flags
    }

    fn do_layout(&mut self, _value: &NodeValue<E>, _ext: &mut E::NodeData, data: &mut Self::ChildData, mut current: Rect, _flags: DirtyFlags) -> Rect {
        data.x.map(|v| current.x = v);
        data.y.map(|v| current.y = v);
        data.width.map(|v| current.width = v);
        data.height.map(|v| current.height = v);
        current
    }
}