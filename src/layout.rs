use super::*;

use std::marker::PhantomData;
use std::cell::RefMut;

/// Used to position an element within another element.
///
/// The order of method calls during layout is as followed
///
/// ```ignore
///
/// parent_layout.do_layout(...);
/// current_layout.start_layout(...);
///
/// for child in children {
///     current_layout.do_layout(...);
///     child_layout.start_layout(...);
///
///     // repeat for children
///
///     child_layout.finish_layout(...);
///     current_layout.do_layout_end(...);
/// }
///
/// current_layout.finish_layout(...);
/// parent_layout.do_layout_end(...);
/// ```
pub trait LayoutEngine<E>
    where E: Extension
{
    /// The type of the data that will be stored on child nodes
    type ChildData: 'static;

    /// The name of this layout as it will be referenced in style rules
    fn name() -> &'static str;

    /// Called to register the properties used by the layout.
    ///
    /// # Example
    /// ```ignore
    /// static MY_PROP: StaticKey = StaticKey("my_prop");
    ///
    /// // ...
    ///
    /// fn style_properties<'a, F>(mut prop: F)
    ///     where F: FnMut(StaticKey) + 'a
    /// {
    ///     prop(MY_PROP);
    /// }
    ///
    /// ```
    fn style_properties<'a, F>(prop: F)
        where F: FnMut(StaticKey) + 'a;

    /// Creates a new child data to be stored on a node
    fn new_child_data() -> Self::ChildData;

    /// Called to apply a given style rule on a node
    ///
    /// Its recomended to use the `eval!` macro to check for relevant properties
    /// as it also skips ones that have already been set by a another rule.
    ///
    /// # Example
    /// ```ignore
    ///
    /// fn update_data(&mut self, styles: &Styles<E>, nc: &NodeChain<E>, rule: &Rule<E>) -> DirtyFlags {
    ///     let mut flags = DirtyFlags::empty();
    ///     eval!(styles, nc, rule.X => val => {
    ///         let new = val.convert();
    ///         if self.x != new {
    ///             self.x = new;
    ///             flags |= DirtyFlags::POSITION;
    ///         }
    ///     });
    ///     flags
    /// }
    /// ```
    fn update_data(&mut self, _styles: &Styles<E>, _nc: &NodeChain<E>, _rule: &Rule<E>) -> DirtyFlags {
        DirtyFlags::empty()
    }

    /// Called to apply a given style rule on a child node
    ///
    /// Its recomended to use the `eval!` macro to check for relevant properties
    /// as it also skips ones that have already been set by a another rule.
    ///
    /// # Example
    /// ```ignore
    ///
    /// fn update_child_data(&mut self, styles: &Styles<E>, nc: &NodeChain<E>, rule: &Rule<E>, data: &mut Self::ChildData) -> DirtyFlags {
    ///     let mut flags = DirtyFlags::empty();
    ///     eval!(styles, nc, rule.X => val => {
    ///         let new = val.convert();
    ///         if data.x != new {
    ///             data.x = new;
    ///             flags |= DirtyFlags::POSITION;
    ///         }
    ///     });
    ///     flags
    /// }
    /// ```
    fn update_child_data(&mut self, _styles: &Styles<E>, _nc: &NodeChain<E>, _rule: &Rule<E>, _data: &mut Self::ChildData) -> DirtyFlags {
        DirtyFlags::empty()
    }

    /// Called after applying all relevant rules to reset any properties that
    /// weren't set.
    ///
    /// This is needed because a node could have a property set previously and
    /// then later (e.g. when a property is changed) no longer have it set.
    /// Due to it no longer being set `update_data` would not be called for
    /// that property leaving it stuck with its previous value.
    ///
    /// `used_keys` will contain every property key that was used by rules
    /// in this update, if the key isn't in this set it should be reset.
    fn reset_unset_data(&mut self, _used_keys: &FnvHashSet<StaticKey>) -> DirtyFlags {
        DirtyFlags::empty()
    }
    /// Called after applying all relevant rules to a child reset any properties
    /// that weren't set.
    ///
    /// This is needed because a node could have a property set previously and
    /// then later (e.g. when a property is changed) no longer have it set.
    /// Due to it no longer being set `update_data` would not be called for
    /// that property leaving it stuck with its previous value.
    ///
    /// `used_keys` will contain every property key that was used by rules
    /// in this update, if the key isn't in this set it should be reset.
    fn reset_unset_child_data(&mut self, _used_keys: &FnvHashSet<StaticKey>, _data: &mut Self::ChildData) -> DirtyFlags {
        DirtyFlags::empty()
    }

    /// Called to check the parent node's dirty flags to update its own dirty flags
    fn check_parent_flags(&mut self, _flags: DirtyFlags) -> DirtyFlags {
        DirtyFlags::empty()
    }
    /// Called to check the child nodes' dirty flags to update its own dirty flags
    fn check_child_flags(&mut self, _flags: DirtyFlags) -> DirtyFlags {
        DirtyFlags::empty()
    }

    /// Begins the layout for this node
    ///
    /// Called after the parent node's layout has called its `do_layout` method
    fn start_layout(&mut self, _ext: &mut E::NodeData, current: Rect, _flags: DirtyFlags, _children: ChildAccess<Self, E>) -> Rect {
        current
    }

    /// Begins the layout for a child node of this layout
    ///
    /// Called before the child node's layout's `start_layout` method
    fn do_layout(&mut self, _value: &NodeValue<E>, _ext: &mut E::NodeData, _data: &mut Self::ChildData, current: Rect, _flags: DirtyFlags) -> Rect {
        current
    }

    /// Ends the layout for this child node
    ///
    /// Called after all the child nodes have had their layout called
    fn do_layout_end(&mut self, _value: &NodeValue<E>, _ext: &mut E::NodeData, _data: &mut Self::ChildData, current: Rect, _flags: DirtyFlags) -> Rect {
        current
    }
    /// Ends the layout for this node
    ///
    /// Called after all the child nodes have had `do_layout(_end)` called
    fn finish_layout(&mut self, _ext: &mut E::NodeData, current: Rect, _flags: DirtyFlags, _children: ChildAccess<Self, E>) -> Rect {
        current
    }
}

/// Provides access to a child node and its stored layout data
pub struct ChildAccess<'a, L: LayoutEngine<E> + ?Sized, E: Extension + 'a> {
    _l: PhantomData<L>,
    nodes: &'a [Node<E>],
}

/// Helper struct to split a `RefMut` on a `NodeInner` whilst
/// `RefMut::split` is unstable.
pub struct NodeAccess<'a, L: LayoutEngine<E> + ?Sized, E: Extension + 'a> {
    node: RefMut<'a, NodeInner<E>>,
    _l: PhantomData<L>,
}

impl <'a, L, E> NodeAccess<'a, L, E>
    where L: LayoutEngine<E>,
          E: Extension
{
    /// Splits this node access into its value and the data stored
    /// on it for this layout.
    #[inline]
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
    /// Returns the number of child nodes
    #[inline]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the child's size, flags and data for the given
    /// index if any.
    #[inline]
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

/// The "x" static key used by the absolute layout
///
/// This should be used if you wish to use "x" in your
/// own layouts due to the fact that two static strings
/// across crates/modules don't always point to the same
/// value which is a requirement for static keys.
pub static X: StaticKey = StaticKey("x");
/// The "y" static key used by the absolute layout
///
/// This should be used if you wish to use "x" in your
/// own layouts due to the fact that two static strings
/// across crates/modules don't always point to the same
/// value which is a requirement for static keys.
pub static Y: StaticKey = StaticKey("y");
/// The "width" static key used by the absolute layout
///
/// This should be used if you wish to use "x" in your
/// own layouts due to the fact that two static strings
/// across crates/modules don't always point to the same
/// value which is a requirement for static keys.
pub static WIDTH: StaticKey = StaticKey("width");
/// The "height" static key used by the absolute layout
///
/// This should be used if you wish to use "x" in your
/// own layouts due to the fact that two static strings
/// across crates/modules don't always point to the same
/// value which is a requirement for static keys.
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