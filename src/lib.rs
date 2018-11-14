
#![warn(missing_docs)]

extern crate fnv;
extern crate fungui_syntax as syntax;
extern crate ref_filter_map;
extern crate bitflags;

pub mod query;
mod error;
pub use error::Error;
#[macro_use]
mod macros;
#[cfg(any(test, feature="tests"))]
pub mod tests;
mod style;
use style::*;
mod expr;
use expr::*;
mod layout;
use layout::*;

pub use layout::{
    LayoutEngine, ChildAccess,
    X, Y, WIDTH, HEIGHT
};

pub use style::{Rule, Styles};
// TODO: Really shouldn't need this
pub use fnv::FnvHashSet;

use fnv::FnvHashMap;
use std::rc::{Rc, Weak};
use std::cell::{Ref, RefMut, RefCell};
use std::any::Any;
use std::hash::{Hash, Hasher};
use bitflags::bitflags;
pub use syntax::{format_error, format_parse_error};

pub type SResult<'a, T> = Result<T, Error<'a>>;

/// An unchanging key
#[derive(Clone, Copy, Debug, Eq)]
pub struct StaticKey(pub &'static str);

impl PartialEq for StaticKey {
    fn eq(&self, other: &StaticKey) -> bool {
        use std::ptr;
        ptr::eq(self.0, other.0)
    }
}

impl Hash for StaticKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.0 as *const str).hash(state)
    }
}

bitflags! {
    pub struct DirtyFlags: u32 {
        const POSITION = 0b0000_0001;
        const SIZE     = 0b0000_0010;
        const SCROLL   = 0b0000_0100;
        const LAYOUT   = 0b0000_1000;
        const TEXT     = 0b0001_0000;
        const CHILDREN = 0b0010_0000;

        // Extra ones for layouts to use
        const LAYOUT_1 = 0b0000_1000_0000_0000_0000_0000_0000_0000;
        const LAYOUT_2 = 0b0000_0100_0000_0000_0000_0000_0000_0000;
        const LAYOUT_3 = 0b0000_0010_0000_0000_0000_0000_0000_0000;
        const LAYOUT_4 = 0b0000_0001_0000_0000_0000_0000_0000_0000;
        const LAYOUT_ALL   = Self::LAYOUT_1.bits | Self::LAYOUT_2.bits | Self::LAYOUT_3.bits | Self::LAYOUT_4.bits;
        // Extra ones for extensions to use
        const EXT_1 = 0b1000_0000_0000_0000_0000_0000_0000_0000;
        const EXT_2 = 0b0100_0000_0000_0000_0000_0000_0000_0000;
        const EXT_3 = 0b0010_0000_0000_0000_0000_0000_0000_0000;
        const EXT_4 = 0b0001_0000_0000_0000_0000_0000_0000_0000;
        const EXT_ALL   = Self::EXT_1.bits | Self::EXT_2.bits | Self::EXT_3.bits | Self::EXT_4.bits;
    }
}

pub trait Extension {
    type NodeData: Sized;
    type Value: Clone + PartialEq + Sized;

    fn new_data() -> Self::NodeData;

    fn style_properties<'a, F>(prop: F)
        where F: FnMut(StaticKey) + 'a;

    fn update_data(styles: &Styles<Self>, nc: &NodeChain<Self>, rule: &Rule<Self>, data: &mut Self::NodeData) -> DirtyFlags
        where Self: Sized;
    fn reset_unset_data(used_keys: &FnvHashSet<StaticKey>, data: &mut Self::NodeData) -> DirtyFlags;
    fn check_flags(_data: &mut Self::NodeData, _flags: DirtyFlags) { }
}

/// Stores loaded nodes and manages the layout.
pub struct Manager<E: Extension> {
    // Has no parent, is the parent for all base nodes
    // in the system
    root: Node<E>,
    styles: Styles<E>,
    last_size: (i32, i32),
    dirty: bool,
}

static CLIP_OVERFLOW: StaticKey = StaticKey("clip_overflow");
static SCROLL_X: StaticKey = StaticKey("scroll_x");
static SCROLL_Y: StaticKey = StaticKey("scroll_y");
static LAYOUT: StaticKey = StaticKey("layout");

impl<E: Extension> Manager<E> {
    /// Creates a new manager with an empty root node.
    pub fn new() -> Manager<E> {
        let mut static_keys = FnvHashMap::default();
        {
            let mut prop = |key: StaticKey| {static_keys.insert(key.0, key);};
            prop(CLIP_OVERFLOW);
            prop(SCROLL_X);
            prop(SCROLL_Y);
            prop(LAYOUT);
            E::style_properties(prop);
        }
        let mut m = Manager {
            root: Node::root(),
            styles: Styles {
                _ext: ::std::marker::PhantomData,
                static_keys,
                rules: Rules::new(),
                funcs: FnvHashMap::default(),
                layouts: FnvHashMap::default(),
                next_rule_id: 0,
                used_keys: FnvHashSet::default(),
            },
            last_size: (0, 0),
            dirty: true,
        };
        m.add_layout_engine(AbsoluteLayout::default);

        m
    }

    /// Adds a new function that can be used to create a layout engine.
    ///
    /// A layout engine is used to position elements within an element.
    ///
    /// The layout engine can be selected by using the `layout` attribute.
    pub fn add_layout_engine<F, L>(&mut self, creator: F)
    where
        F: Fn() -> L + 'static,
        L: LayoutEngine<E> + 'static,
    {
        L::style_properties(|key| {self.styles.static_keys.insert(key.0, key);});
        self.styles.layouts.insert(L::name(), Box::new(move || Box::new(creator())));
    }

    /// Add a function that can be called by styles
    pub fn add_func_raw<F>(&mut self, name: &'static str, func: F)
    where
        F: for<'a> Fn(&mut (Iterator<Item=Result<Value<E>, Error<'a>>> + 'a)) -> Result<Value<E>, Error<'a>> + 'static,
    {
        let key = self.styles.static_keys.entry(name).or_insert(StaticKey(name));
        self.styles.funcs.insert(*key, Box::new(func));
    }

    /// Adds the node to the root node of this manager.
    ///
    /// The node is created from the passed string.
    /// See [`add_node_str`](struct.Node.html#from_str)
    pub fn add_node_str<'a>(&mut self, node: &'a str) -> Result<(), syntax::PError<'a>> {
        self.add_node(Node::from_str(node)?);
        Ok(())
    }

    /// Adds the node to the root node of this manager
    pub fn add_node(&mut self, node: Node<E>) {
        self.root.add_child(node);
    }

    /// Removes the node from the root node of this manager
    pub fn remove_node(&mut self, node: Node<E>) {
        self.root.remove_child(node);
    }

    /// Starts a query from the root of this manager
    pub fn query(&self) -> query::Query<E> {
        query::Query::new(self.root.clone())
    }

    /// Starts a query looking for elements at the target
    /// location.
    pub fn query_at(&self, x: i32, y: i32) -> query::Query<'static, E> {
        query::Query {
            root: self.root.clone(),
            rules: Vec::new(),
            location: Some(query::AtLocation { x: x, y: y }),
        }
    }

    /// Loads a set of styles from the given string.
    ///
    /// If a set of styles with the same name is already loaded
    /// then this will replace them.
    pub fn load_styles<'a>(
        &mut self,
        name: &str,
        style_rules: &'a str,
    ) -> Result<(), syntax::PError<'a>> {
        let styles = syntax::style::Document::parse(style_rules)?;
        self.styles.load_styles(name, styles)?;
        self.dirty = true;
        Ok(())
    }

    /// Removes the set of styles with the given name
    pub fn remove_styles(&mut self, name: &str) {
        self.styles.rules.remove_all_by_name(name);
        self.dirty = true;
    }

    /// Positions the nodes in this manager.
    pub fn layout(&mut self, width: i32, height: i32) {
        let size = (width, height);
        let flags = if self.last_size != size {
            self.last_size = size;
            DirtyFlags::SIZE
        } else {
            DirtyFlags::empty()
        };

        let mut inner = self.root.inner.borrow_mut();
        inner.draw_rect = Rect{x: 0, y: 0, width, height};

        let p = NodeChain {
            parent: None,
            value: NCValue::Element("root"),
            draw_rect: inner.draw_rect,
            properties: &FnvHashMap::default(),
        };

        let mut layout = AbsoluteLayout::default();

        // This is a loop due to the `parent_X` support requiring
        // the layout to be computed so it can be used in style rules
        // creating a chicken/egg problem. If they aren't used then
        // this will only execute once.
        loop {
            let mut properties_changed = false;

            if let NodeValue::Element(ref v) = inner.value {
                for c in &v.children {
                    c.do_update(&mut self.styles, &p, &mut layout, self.dirty, flags == DirtyFlags::SIZE, flags);
                }

                for c in &v.children {
                    properties_changed |= c.layout(&self.styles, &mut layout);
                }
            }

            self.dirty = false;
            if !properties_changed {
                break;
            }
        }
    }

    /// Renders the nodes in this manager by passing the
    /// layout and styles to the passed visitor.
    pub fn render<V>(&mut self, visitor: &mut V)
    where
        V: RenderVisitor<E>,
    {
        self.root.render(visitor);
    }
}

/// The position and size of an element
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Called for every element in a manager to allow them to
/// be rendered.
pub trait RenderVisitor<E: Extension> {
    fn visit(&mut self, node: &mut NodeInner<E>);
    fn visit_end(&mut self, node: &mut NodeInner<E>);
}

/// A node representing an element.
///
/// Can be cloned to duplicate the reference to the node.
pub struct Node<E: Extension> {
    inner: Rc<RefCell<NodeInner<E>>>,
}

impl<E: Extension> Clone for Node<E> {
    fn clone(&self) -> Self {
        Node {
            inner: self.inner.clone(),
        }
    }
}

#[macro_export]
macro_rules! eval {
    ($styles:expr, $n:expr, $rule:ident.$key:expr => $ret:ident => $ok:block) => {
        if !$styles.key_was_used(&$key) {
            if let Some(e) = $rule.styles.get(&$key) {
                match e.eval($styles, &$n) {
                    Ok($ret) => $ok,
                    Err(err) => {
                        // TODO: Collect errors for the user to display
                        // instead of printing
                        println!("Failed to evalulate expression ({}): {:?}", e, err);
                    }
                }
            }
        }
    };
}

impl<E: Extension> Node<E> {

    fn do_update(
        &self,
        styles: &mut Styles<E>,
        parent: &NodeChain<E>,
        parent_layout: &mut dyn BoxLayoutEngine<E>,
        mut styles_updated: bool, mut parent_dirty: bool,
        parent_flags: DirtyFlags,
    ) -> DirtyFlags
    {
        use std::mem::replace;

        let inner: &mut _ = &mut *self.inner.borrow_mut();
        let props_dirty = replace(&mut inner.properties_changed, false);
        let rules_dirty = replace(&mut inner.rules_dirty, false);
        inner.dirty_flags = DirtyFlags::empty();
        if inner.text_changed {
            inner.dirty_flags |= DirtyFlags::TEXT;
        }
        if rules_dirty {
            inner.dirty_flags |= DirtyFlags::CHILDREN;
        }

        if styles_updated || rules_dirty {
            styles_updated = true;
            parent_dirty = true;
            inner.possible_rules.clear();
            let c = NodeChain {
                parent: Some(parent),
                value: inner.value.as_chain(),
                draw_rect: inner.draw_rect,
                properties: &inner.properties,
            };
            styles.rules.get_possible_matches(&c, &mut inner.possible_rules);
        }
        if parent_dirty || props_dirty {
            parent_dirty = true;
            let c = NodeChain {
                parent: Some(parent),
                value: inner.value.as_chain(),
                draw_rect: inner.draw_rect,
                properties: &inner.properties,
            };
            styles.used_keys.clear();
            inner.uses_parent_size = false;
            for rule in inner.possible_rules.iter().rev() {
                if rule.test(&c) {
                    inner.uses_parent_size |= rule.uses_parent_size;
                    eval!(styles, c, rule.LAYOUT => val => {
                        let new = val.convert::<String>();
                        let new = new.as_ref().map(|v| v.as_str())
                            .unwrap_or("absolute");
                        if new != inner.layout.name() {
                            if let Some(nl) = styles.layouts.get(new) {
                                inner.layout = nl();
                                inner.dirty_flags |= DirtyFlags::POSITION | DirtyFlags::SIZE | DirtyFlags::LAYOUT;
                            }
                        }
                    });
                    // TODO: Error/warn on incorrect types?
                    eval!(styles, c, rule.SCROLL_X => val => {
                        let new = val.convert().unwrap_or(0.0);
                        if inner.scroll_position.0 != new {
                            inner.scroll_position.0 = new;
                            inner.dirty_flags |= DirtyFlags::SCROLL;
                        }
                    });
                    eval!(styles, c, rule.SCROLL_Y => val => {
                        let new = val.convert().unwrap_or(0.0);
                        if inner.scroll_position.1 != new {
                            inner.scroll_position.1 = new;
                            inner.dirty_flags |= DirtyFlags::SCROLL;
                        }
                    });
                    eval!(styles, c, rule.CLIP_OVERFLOW => val => {
                        inner.clip_overflow = val.convert().unwrap_or(false);
                    });
                    inner.dirty_flags |= E::update_data(styles, &c, rule, &mut inner.ext);
                    inner.dirty_flags |= inner.layout.update_data(styles, &c, rule);
                    inner.dirty_flags |= parent_layout.update_child_data(styles, &c, rule, &mut inner.parent_data);

                    styles.used_keys.extend(rule.styles.keys());
                }
            }
            if !styles.used_keys.contains(&CLIP_OVERFLOW) {
                inner.clip_overflow = false;
            }
            if !styles.used_keys.contains(&SCROLL_X) {
                inner.scroll_position.0 = 0.0;
                inner.dirty_flags |= DirtyFlags::SCROLL;
            }
            if !styles.used_keys.contains(&SCROLL_Y) {
                inner.scroll_position.1 = 0.0;
                inner.dirty_flags |= DirtyFlags::SCROLL;
            }
            inner.dirty_flags |= E::reset_unset_data(&styles.used_keys, &mut inner.ext);
            inner.dirty_flags |= inner.layout.reset_unset_data(&styles.used_keys);
            inner.dirty_flags |= parent_layout.reset_unset_child_data(&styles.used_keys, &mut inner.parent_data);

        }
        inner.dirty_flags |= inner.layout.check_parent_flags(parent_flags);
        let mut child_flags = DirtyFlags::empty();
        let p = NodeChain {
            parent: Some(parent),
            value: inner.value.as_chain(),
            draw_rect: inner.draw_rect,
            properties: &inner.properties,
        };
        if let NodeValue::Element(ref v) = inner.value {
            for c in &v.children {
                child_flags |= c.do_update(styles, &p, &mut *inner.layout, styles_updated, parent_dirty, inner.dirty_flags);
            }
        }
        inner.dirty_flags |= inner.layout.check_child_flags(child_flags);

        E::check_flags(&mut inner.ext, inner.dirty_flags);

        inner.dirty_flags
    }

    fn layout(
        &self,
        styles: &Styles<E>,
        parent_layout: &mut dyn BoxLayoutEngine<E>,
    ) -> bool {
        let inner: &mut _ = &mut *self.inner.borrow_mut();
        inner.done_layout = true;
        let nodes = if let NodeValue::Element(ref v) = inner.value {
            v.children.as_slice()
        } else {
            &[]
        };
        inner.draw_rect = parent_layout.do_layout(&inner.value, &mut inner.ext, &mut inner.parent_data, inner.draw_rect, inner.dirty_flags);
        inner.draw_rect = inner.layout.start_layout(&mut inner.ext, inner.draw_rect, inner.dirty_flags, nodes);

        let mut properties_changed = false;
        for c in nodes {
            properties_changed |= c.layout(styles, &mut *inner.layout);
        }
        inner.draw_rect = inner.layout.finish_layout(&mut inner.ext, inner.draw_rect, inner.dirty_flags, nodes);
        inner.draw_rect = parent_layout.do_layout_end(&inner.value, &mut inner.ext, &mut inner.parent_data, inner.draw_rect, inner.dirty_flags);

        if inner.draw_rect != inner.prev_rect {
            for c in nodes {
                let mut c = c.inner.borrow_mut();
                if c.uses_parent_size {
                    c.properties_changed = true;
                    properties_changed = true;
                }
            }
        }
        inner.prev_rect = inner.draw_rect;
        properties_changed
    }

    fn render<V>(&self, visitor: &mut V)
    where
        V: RenderVisitor<E>,
    {
        let inner: &mut _ = &mut *self.inner.borrow_mut();
        visitor.visit(inner);
        if let NodeValue::Element(ref v) = inner.value {
            for c in &v.children {
                c.render(visitor);
            }
        }
        visitor.visit_end(inner);
    }

    /// Creates a new element with the given name.
    pub fn new<S>(name: S) -> Node<E>
    where
        S: Into<String>,
    {
        Node {
            inner: Rc::new(RefCell::new(NodeInner {
                value: NodeValue::Element(Element {
                    name: name.into(),
                    children: Vec::new(),
                }),
                .. Default::default()
            })),
        }
    }

    /// Creates a new text node with the given text.
    pub fn new_text<S>(text: S) -> Node<E>
    where
        S: Into<String>,
    {
        Node {
            inner: Rc::new(RefCell::new(NodeInner {
                value: NodeValue::Text(text.into()),
                .. Default::default()
            })),
        }
    }

    pub fn borrow(&self) -> Ref<NodeInner<E>> {
        self.inner.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<NodeInner<E>> {
        self.inner.borrow_mut()
    }

    /// Adds the passed node as a child to this node
    /// before other child nodes.
    ///
    /// Returns true if the node was added
    pub fn add_child_first(&self, node: Node<E>) -> bool {
        if node.inner.borrow().parent.is_some() {
            return false;
        }
        if let NodeValue::Element(ref mut e) = self.inner.borrow_mut().value {
            {
                let mut inner = node.inner.borrow_mut();
                inner.parent = Some(Rc::downgrade(&self.inner));
                inner.rules_dirty = true;
            }
            e.children.insert(0, node);
            true
        } else {
            false
        }
    }

    /// Adds the passed node as a child to this node.
    ///
    /// Returns true if the node was added
    pub fn add_child(&self, node: Node<E>) -> bool {
        if node.inner.borrow().parent.is_some() {
            return false;
        }
        if let NodeValue::Element(ref mut e) = self.inner.borrow_mut().value {
            {
                let mut inner = node.inner.borrow_mut();
                inner.parent = Some(Rc::downgrade(&self.inner));
                inner.rules_dirty = true;
            }
            e.children.push(node);
            true
        } else {
            false
        }
    }

    /// Removes the passed node as a child from this node.
    ///
    /// Returns true if the node was removed
    pub fn remove_child(&self, node: Node<E>) -> bool {
        if !node.inner
            .borrow()
            .parent
            .as_ref()
            .and_then(|v| v.upgrade())
            .map_or(false, |v| Rc::ptr_eq(&v, &self.inner)) {
            return false;
        }
        let inner: &mut NodeInner<_> = &mut *self.inner.borrow_mut();
        if let NodeValue::Element(ref mut e) = inner.value {
            e.children.retain(|v| !Rc::ptr_eq(&v.inner, &node.inner));
            {
                let mut inner = node.inner.borrow_mut();
                inner.parent = None;
                inner.rules_dirty = true;
            }
            true
        } else {
            false
        }
    }

    /// Returns a vector containing the child nodes of this
    /// node.
    pub fn children(&self) -> Vec<Node<E>> {
        if let NodeValue::Element(ref e) = self.inner.borrow().value {
            Clone::clone(&e.children)
        } else {
            Vec::new()
        }
    }

    /// Returns the parent node of this node.
    pub fn parent(&self) -> Option<Node<E>> {
        let inner = self.inner.borrow();
        inner
            .parent
            .as_ref()
            .and_then(|v| v.upgrade())
            .map(|v| Node { inner: v })
    }

    /// Returns the name of the node if it has one
    pub fn name(&self) -> Option<String> {
        let inner = self.inner.borrow();
        match inner.value {
            NodeValue::Element(ref e) => Some(e.name.clone()),
            NodeValue::Text(_) => None,
        }
    }

    /// Returns whether the passed node points to the same node
    /// as this one
    pub fn is_same(&self, other: &Node<E>) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }

    /// Returns the text of the node if it is a text node.
    pub fn text(&self) -> Option<Ref<str>> {
        let inner = self.inner.borrow();
        ref_filter_map::ref_filter_map(inner, |v|
            if let NodeValue::Text(ref t) = v.value {
                Some(t.as_str())
            } else {
                None
            }
        )
    }

    /// Sets the text of the node if it is a text node.
    pub fn set_text<S>(&self, txt: S)
    where
        S: Into<String>,
        String: PartialEq<S>,
    {
        let inner: &mut NodeInner<_> = &mut *self.inner.borrow_mut();
        if let NodeValue::Text(ref mut t) = inner.value {
            if *t != txt{
                *t = txt.into();
                inner.text_changed = true;
            }
        }
    }

    /// Returns whether this node has had its layout computed
    /// at least once
    pub fn has_layout(&self) -> bool {
        self.inner.borrow().done_layout
    }

    /// Returns the raw position of the node.
    ///
    /// This position isn't transformed and is relative
    /// to the parent instead of absolute like `render_position`
    pub fn raw_position(&self) -> Rect {
        self.inner.borrow().draw_rect
    }

    /// Returns the rendering position of the node.
    ///
    /// Useful for IME handling.
    /// Must be called after a `layout` call.
    pub fn render_position(&self) -> Option<Rect> {
        let inner = self.inner.borrow();
        let mut rect = inner.draw_rect;
        let mut cur = inner.parent.as_ref().and_then(|v| v.upgrade());
        while let Some(p) = cur {
            let inner = p.borrow();
            rect.x += inner.scroll_position.0 as i32;
            rect.y += inner.scroll_position.1 as i32;
            if inner.clip_overflow {
                if rect.x < 0 {
                    rect.width += rect.x;
                    rect.x = 0;
                }
                if rect.y < 0 {
                    rect.height += rect.y;
                    rect.y = 0;
                }
                if rect.x + rect.width >= inner.draw_rect.width {
                    rect.width -= (rect.x + rect.width) - inner.draw_rect.width;
                }
                if rect.y + rect.height >= inner.draw_rect.height {
                    rect.height -= (rect.y + rect.height) - inner.draw_rect.height;
                }
            }
            if rect.width <= 0 || rect.height <= 0 {
                return None;
            }

            rect.x += inner.draw_rect.x;
            rect.y += inner.draw_rect.y;
            cur = inner.parent.as_ref().and_then(|v| v.upgrade());
        }
        Some(rect)
    }

    /// Removes the property on the node.
    pub fn remove_property(&self, key: &str) {
        let mut inner = self.inner.borrow_mut();
        inner.properties.remove(key);
    }

    #[inline]
    pub fn get_property<V>(&self, key: &str) -> Option<V>
        where V: ConvertValue<E>
    {
        let inner = self.inner.borrow();
        inner.get_property::<V>(key)
    }

    #[inline]
    pub fn get_property_ref<V>(&self, key: &str) -> Option<Ref<V::RefType>>
        where V: ConvertValue<E>
    {
        let inner = self.inner.borrow();
        ref_filter_map::ref_filter_map(
            inner,
            |v| v.get_property_ref::<V>(key)
        )
    }

    pub fn set_property<V>(&self, key: &str, v: V)
        where V: ConvertValue<E>
    {
        let mut inner = self.inner.borrow_mut();
        inner.properties_changed = true;
        inner.properties.insert(key.into(), V::to_value(v));
    }

    pub fn raw_set_property<V>(&self, key: &str, v: V)
        where V: ConvertValue<E>
    {
        let mut inner = self.inner.borrow_mut();
        inner.properties.insert(key.into(), V::to_value(v));
    }

    /// Creates a weak reference to this node.
    pub fn weak(&self) -> WeakNode<E> {
        WeakNode {
            inner: Rc::downgrade(&self.inner),
        }
    }

    /// Begins a query on this node
    pub fn query(&self) -> query::Query<E> {
        query::Query::new(self.clone())
    }

    /// Creates a node from a string
    pub fn from_str(s: &str) -> Result<Node<E>, syntax::PError> {
        syntax::desc::Document::parse(s).map(|v| Node::from_document(v))
    }

    /// Creates a node from a parsed document.
    pub fn from_document(desc: syntax::desc::Document) -> Node<E> {
        Node::from_doc_element(desc.root)
    }

    fn from_doc_text(
        desc: &str,
        properties: FnvHashMap<syntax::Ident, syntax::desc::ValueType>,
    ) -> Node<E> {
        let text = unescape(desc);
        Node {
            inner: Rc::new(RefCell::new(NodeInner {
                value: NodeValue::Text(text),
                properties: properties
                    .into_iter()
                    .map(|(n, v)| (n.name.into(), Value::from(v)))
                    .collect(),
                .. Default::default()
            })),
        }
    }

    fn from_doc_element(desc: syntax::desc::Element) -> Node<E> {
        let node = Node {
            inner: Rc::new(RefCell::new(NodeInner {
                value: NodeValue::Element(Element {
                    name: desc.name.name.into(),
                    children: Vec::with_capacity(desc.nodes.len()),
                }),
                properties: desc.properties
                    .into_iter()
                    .map(|(n, v)| (n.name.into(), Value::from(v)))
                    .collect(),
                .. Default::default()
            })),
        };

        for c in desc.nodes.into_iter().map(|n| match n {
            syntax::desc::Node::Element(e) => Node::from_doc_element(e),
            syntax::desc::Node::Text(t, _, props) => Node::from_doc_text(t, props),
        }) {
            node.add_child(c);
        }

        node
    }

    fn root() -> Node<E> {
        Node {
            inner: Rc::new(RefCell::new(NodeInner {
                value: NodeValue::Element(Element {
                    name: "root".into(),
                    children: Vec::new(),
                }),
                .. Default::default()
            })),
        }
    }
}

fn unescape(v: &str) -> String {
    let mut text = String::new();
    let mut special = false;
    for c in v.chars() {
        if special {
            match c {
                't' => text.push('\t'),
                'n' => text.push('\n'),
                'r' => text.push('\r'),
                _ => text.push(c),
            }
            special = false;
            continue;
        }
        if c == '\\' {
            special = true;
        } else {
            text.push(c);
        }
    }
    text
}

/// A weak reference to a node.
pub struct WeakNode<E: Extension> {
    inner: Weak<RefCell<NodeInner<E>>>,
}
impl<E: Extension> WeakNode<E> {
    /// Tries to upgrade this weak reference into a strong one.
    ///
    /// Fails if there isn't any strong references to the node.
    pub fn upgrade(&self) -> Option<Node<E>> {
        self.inner.upgrade().map(|v| Node { inner: v })
    }
}

impl<E: Extension> Clone for WeakNode<E> {
    fn clone(&self) -> Self {
        WeakNode {
            inner: self.inner.clone(),
        }
    }
}

pub struct NodeInner<E: Extension> {
    parent: Option<Weak<RefCell<NodeInner<E>>>>,
    properties: FnvHashMap<String, Value<E>>,
    properties_changed: bool,
    possible_rules: Vec<Rc<Rule<E>>>,
    done_layout: bool,
    // Set when added/removed from a node
    rules_dirty: bool,
    dirty_flags: DirtyFlags,
    pub value: NodeValue<E>,
    pub text_changed: bool,
    layout: Box<dyn BoxLayoutEngine<E>>,
    parent_data: Box<dyn Any>,
    uses_parent_size: bool,
    prev_rect: Rect,
    pub draw_rect: Rect,
    /// The scroll offset of all elements inside this one
    pub scroll_position: (f32, f32),
    /// Whether this element clips child elements that overflow
    /// its bounds
    pub clip_overflow: bool,
    /// The location that this element should be drawn at as
    /// decided by the layout engine
    pub draw_position: Rect,
    /// Extension provided data
    pub ext: E::NodeData,
}

impl <E> Default for NodeInner<E>
    where E: Extension
{
    fn default() -> NodeInner<E> {
        NodeInner {
            parent: None,
            layout: Box::new(AbsoluteLayout::default()),
            parent_data: Box::new(AbsoluteLayoutChild::default()),
            value: NodeValue::Text(String::new()),
            properties: FnvHashMap::default(),
            properties_changed: true,
            possible_rules: Vec::new(),
            done_layout: false,
            rules_dirty: true,
            text_changed: false,
            dirty_flags: DirtyFlags::empty(),
            uses_parent_size: false,
            prev_rect: Rect{x: 0, y: 0, width: 0, height: 0},
            draw_rect: Rect{x: 0, y: 0, width: 0, height: 0},
            scroll_position: (0.0, 0.0),
            clip_overflow: false,
            draw_position: Rect{x: 0, y: 0, width: 0, height: 0},
            ext: E::new_data(),
        }
    }
}

impl <E> NodeInner<E>
    where E: Extension
{
    #[inline]
    fn get_property_impl<V>(props: &FnvHashMap<String, Value<E>>, key: &str) -> Option<V>
        where V: ConvertValue<E>
    {
        props.get(key)
            .cloned()
            .and_then(|v| V::from_value(v))
    }

    #[inline]
    pub fn get_property<V>(&self, key: &str) -> Option<V>
        where V: ConvertValue<E>
    {
        Self::get_property_impl::<V>(&self.properties, key)
    }

    #[inline]
    pub fn get_property_ref_impl<'a, V>(props: &'a FnvHashMap<String, Value<E>>, key: &str) -> Option<&'a V::RefType>
        where V: ConvertValue<E>
    {
        props.get(key)
            .and_then(|v| V::from_value_ref(v))
    }

    #[inline]
    pub fn get_property_ref<V>(&self, key: &str) -> Option<&V::RefType>
        where V: ConvertValue<E>
    {
        Self::get_property_ref_impl::<V>(&self.properties, key)
    }

    pub fn text(&self) -> Option<&str> {
        match self.value {
            NodeValue::Element(_) => None,
            NodeValue::Text(ref t) => Some(t.as_str()),
        }
    }
}

pub enum NodeValue<E: Extension> {
    Element(Element<E>),
    Text(String),
}

impl <E: Extension> NodeValue<E> {

    pub fn text(&self) -> Option<&str> {
        match self {
            NodeValue::Element(_) => None,
            NodeValue::Text(ref t) => Some(t.as_str()),
        }
    }
}

pub struct Element<E: Extension> {
    name: String,
    children: Vec<Node<E>>,
}

pub struct NodeChain<'a, E: Extension + 'a> {
    parent: Option<&'a NodeChain<'a, E>>,
    value: NCValue<'a>,
    draw_rect: Rect,
    properties: &'a FnvHashMap<String, Value<E>>,
}

impl <'a, E> NodeChain<'a, E>
    where E: Extension
{
    pub fn text(&self) -> Option<&'a str> {
        match self.value {
            NCValue::Text(v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Debug)]
enum NCValue<'a> {
    Text(&'a str),
    Element(&'a str),
}
impl <E: Extension> NodeValue<E> {
    fn as_chain(&self) -> NCValue {
        match *self {
            NodeValue::Text(ref t) => NCValue::Text(t.as_str()),
            NodeValue::Element(ref e) => NCValue::Element(e.name.as_str()),
        }
    }
}

/// A value that can be used as a style attribute
#[derive(Debug)]
pub enum Value<E: Extension> {
    Boolean(bool),
    Integer(i32),
    Float(f64),
    String(String),
    ExtValue(E::Value),
}

impl <E> Value<E>
    where E: Extension
{
    pub fn convert<V>(self) -> Option<V>
        where V: ConvertValue<E>
    {
        V::from_value(self)
    }

    pub fn convert_ref<V>(&self) -> Option<&V::RefType>
        where V: ConvertValue<E>
    {
        V::from_value_ref(self)
    }
}

impl <E> Clone for Value<E>
    where E: Extension
{
    fn clone(&self) -> Value<E> {
        match *self {
            Value::Boolean(v) => Value::Boolean(v),
            Value::Integer(v) => Value::Integer(v),
            Value::Float(v) => Value::Float(v),
            Value::String(ref v) => Value::String(v.clone()),
            Value::ExtValue(ref v) => Value::ExtValue(v.clone()),
        }
    }
}

impl <E> PartialEq for Value<E>
    where E: Extension
{
    fn eq(&self, rhs: &Value<E>) -> bool {
        use Value::*;
        match (self, rhs) {
            (&Boolean(a), &Boolean(b)) => a == b,
            (&Integer(a), &Integer(b)) => a == b,
            (&Float(a), &Float(b)) => a == b,
            (&String(ref a), &String(ref b)) => a == b,
            (&ExtValue(ref a), &ExtValue(ref b)) => a == b,
            _ => false,
        }
    }
}

impl <'a, E> From<syntax::desc::ValueType<'a>> for Value<E>
    where E: Extension
{
    fn from(v: syntax::desc::ValueType<'a>) -> Value<E> {
        match v.value {
            syntax::desc::Value::Boolean(val) => Value::Boolean(val),
            syntax::desc::Value::Integer(val) => Value::Integer(val),
            syntax::desc::Value::Float(val) => Value::Float(val),
            syntax::desc::Value::String(val) => Value::String(unescape(val)),
        }
    }
}

pub trait ConvertValue<E: Extension>: Sized {
    type RefType: ?Sized;
    fn from_value(v: Value<E>) -> Option<Self>;
    fn from_value_ref(v: &Value<E>) -> Option<&Self::RefType>;
    fn to_value(v: Self) -> Value<E>;
}

impl <E> ConvertValue<E> for i32
    where E: Extension
{
    type RefType = i32;
    fn from_value(v: Value<E>) -> Option<i32> {
        match v {
            Value::Integer(i) => Some(i),
            Value::Float(f) => Some(f as i32),
            _ => None,
        }
    }
    fn from_value_ref(v: &Value<E>) -> Option<&Self::RefType> {
        match v {
            Value::Integer(i) => Some(i),
            _ => None,
        }
    }
    fn to_value(v: Self) -> Value<E> {
        Value::Integer(v)
    }
}

impl <E> ConvertValue<E> for f64
    where E: Extension
{
    type RefType = f64;
    fn from_value(v: Value<E>) -> Option<f64> {
        match v {
            Value::Integer(i) => Some(i as f64),
            Value::Float(f) => Some(f),
            _ => None,
        }
    }
    fn from_value_ref(v: &Value<E>) -> Option<&Self::RefType> {
        match v {
            Value::Float(f) => Some(f),
            _ => None,
        }
    }
    fn to_value(v: Self) -> Value<E> {
        Value::Float(v)
    }
}

impl <E> ConvertValue<E> for f32
    where E: Extension
{
    type RefType = f64;
    fn from_value(v: Value<E>) -> Option<f32> {
        match v {
            Value::Integer(i) => Some(i as f32),
            Value::Float(f) => Some(f as f32),
            _ => None,
        }
    }
    fn from_value_ref(v: &Value<E>) -> Option<&Self::RefType> {
        match v {
            Value::Float(f) => Some(f),
            _ => None,
        }
    }
    fn to_value(v: Self) -> Value<E> {
        Value::Float(v as f64)
    }
}

impl <E> ConvertValue<E> for bool
    where E: Extension
{
    type RefType = bool;
    fn from_value(v: Value<E>) -> Option<bool> {
        match v {
            Value::Boolean(b) => Some(b),
            _ => None,
        }
    }
    fn from_value_ref(v: &Value<E>) -> Option<&Self::RefType> {
        match v {
            Value::Boolean(b) => Some(b),
            _ => None,
        }
    }
    fn to_value(v: Self) -> Value<E> {
        Value::Boolean(v)
    }
}

impl <E> ConvertValue<E> for String
    where E: Extension
{
    type RefType = str;
    fn from_value(v: Value<E>) -> Option<String> {
        match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }
    fn from_value_ref(v: &Value<E>) -> Option<&Self::RefType> {
        match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }
    fn to_value(v: Self) -> Value<E> {
        Value::String(v)
    }
}
impl <E> ConvertValue<E> for Value<E>
    where E: Extension
{
    type RefType = Value<E>;
    fn from_value(v: Value<E>) -> Option<Value<E>> {
        Some(v)
    }
    fn from_value_ref(v: &Value<E>) -> Option<&Self::RefType> {
        Some(v)
    }
    fn to_value(v: Self) -> Value<E> {
        v
    }
}