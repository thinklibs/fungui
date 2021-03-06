//! FunGUI is a UI layout system that seperates the description of the interface and
//! the styling/layout.
//!
//! In FunGUI there are two main systems that come into play that the
//! rest is built around: Nodes and Styles.
//!
//! # Nodes
//!
//! Nodes are used to describe the user interface without any information
//! about how it should look, only the structure. There are two types of
//! nodes: Elements and Text.
//!
//! Elements are simply a name which could be anything, there are no special
//! names as everything is controled by the style rules. Elements may contain
//! child nodes.
//!
//! Text as the name implies is just text. Unlike elements, text may not
//! have any child nodes.
//!
//! Any node may have properties on it. Properties are used to provide
//! configuration to a node which is useful if you use the same node type
//! multiple times. For example an `url` property may be used on a text
//! node to allow the style rules to color it differently or make it clickable.
//!
//! ## Example
//!
//! An example of the node format:
//!
//! ```rust
//! # extern crate fungui_syntax;
//! # fungui_syntax::desc::Document::parse(r##"
//! alert(level="warning") {
//!     title {
//!         "This is an alert"
//!     }
//!     content {
//!         "If you would like more info click "
//!         "here"(url="http://....")
//!         "."
//!     }
//!     buttons {
//!         button(focused=true) {
//!             "Accept"
//!         }
//!         button {
//!             "Ignore"
//!         }
//!     }
//! }
//! # "##).unwrap();
//! ```
//!
//! # Styles
//!
//! Styles are used to define the behaviour of a node. This can be something like
//! how the node will render or how the node will react to events.
//!
//! Styles apply using matching rules to find what nodes they will apply too. Rules
//! can specific a hierarchy of nodes and what properties the node should have and
//! their values. This allows for a `title` inside an `alert` to act differently to
//! a `title` inside an `window` for example.
//!
//! Once a match is found the style rules are applied to the node. Rules can be a
//! simple constant value or an expression. Expressions perform basic math (`+-/*%`)
//! and boolean operations (`|| && <= ` etc), reference properties that were matched
//! and execute functions. Functions can be used for complex properties instead of
//! spliting them across multiple rules.
//!
//! ## Variables and types
//!
//! Variables are typed and floats/integers are treated as seperate and not casted
//! automatically, this includes constants in style rules as well. For constants
//! defining a number as `5` will be an integer whilst `5.0` will be a float. For
//! variables you can cast using `int(val)` or `float(val)`.
//!
//! ### Special variables
//!
//! There are two special variables that can be used without using them in a matching
//! rule: `parent_width` and `parent_height`. These allow you to size things relative
//! to the parent's size without needing a custom layout to handle it. Whilst these
//! are useful in some cases they do come with a larger cost. In order to handle this
//! the interface may have to re-run the layout system multiple to resolve the variables
//! causing a slowdown however this will generally only happen the first time the
//! node has its layout computed.
//!
//! ## Example
//!
//! An example of the style format:
//!
//! ```rust
//! # extern crate fungui_syntax;
//! # fungui_syntax::style::Document::parse(r##"
//! alert {
//!     center = true,
//!     layout = "rows",
//!     width = 500,
//!     height = 400,
//! }
//! alert(level="warning") {
//!     background_color = rgb(255, 255, 0),
//! }
//!
//! alert > title {
//!     layout = "lined",
//!     width = parent_width,
//! }
//! alert > title > @text {
//!     font_color = rgb(0, 0, 0),
//!     font_size = 24,
//! }
//!
//! alert > content {
//!     layout = "lined",
//!     width = parent_width,
//! }
//! alert > content > @text {
//!     font_color = rgb(0, 0, 0),
//!     font_size = 16,
//! }
//! alert > content > @text(url=url) {
//!     font_color = rgb(0, 120, 0),
//!     on_mouse_up = action("visit", url),
//! }
//! # "##).unwrap();
//! ```
//!
//! # Layouts
//!
//! Layouts take some of the style rules and use that to position and size a node.
//! These can be added via `add_layout_engine` and selected using the `layout` style
//! property.
//!
//! # Extension
//!
//! The `Extension` trait paired with the `RenderVisitor` trait is the main way that
//! is used to actually make the user interface do something. By itself FunGUI only
//! does layout, the extension trait can be used to add events and rendering by adding
//! its own properties to use in style rules. In UniverCity these are things like
//! `image` and `background_color` for rendering and `on_mouse_down` for events where
//! events are lua code defined inline with the styles:
//!
//! ```ignore
//! button {
//!     border_width = border_width(15.0),
//!     border = border_image("ui/button", 15, 15, true),
//!     shadow = shadow(2.0, 2.0, rgba(0, 0, 0, 0.3), 3.0, 0.0, "outset"),
//!     layout = "center",
//!     can_hover = true,
//! }
//!
//! button(theme="blueprint") {
//!     border_width = border_width(15.0),
//!     border = border_image("ui/button", 15, 15, true),
//!     tint = rgba(0, 255, 255, 0.4),
//! }
//! button(on_click=click) {
//!     on_mouse_up = list(click, "init#
//!         audio.play_sound('click')
//!         return true
//!     "),
//! }
//! ```


#![warn(missing_docs)]

extern crate fnv;
extern crate fungui_syntax as syntax;
extern crate ref_filter_map;
extern crate bitflags;

mod query;
pub use query::Query;
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
    NodeAccess,
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

/// An alias for a common return type used in FunGUI
pub type FResult<'a, T> = Result<T, Error<'a>>;

/// An unchanging key
///
/// `Hash` and `Eq` are based on the pointer instead of
/// the value of the string. Its recomended to create this
/// via `static` to make sure that it always points to the
/// same thing.
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
    /// Flags used to mark certain properties as dirty/changed
    pub struct DirtyFlags: u32 {
        /// Marks the node's position as changed
        const POSITION = 0b0000_0001;
        /// Marks the node's size as changed
        const SIZE     = 0b0000_0010;
        /// Marks the node's scroll position as changed
        const SCROLL   = 0b0000_0100;
        /// Marks the node's layout as changed
        const LAYOUT   = 0b0000_1000;
        /// Marks the node's text as changed
        const TEXT     = 0b0001_0000;
        /// Marks the node's children as changed
        const CHILDREN = 0b0010_0000;

        // Extra ones for layouts to use
        /// Extra flag for layouts to use
        const LAYOUT_1 = 0b0000_1000_0000_0000_0000_0000_0000_0000;
        /// Extra flag for layouts to use
        const LAYOUT_2 = 0b0000_0100_0000_0000_0000_0000_0000_0000;
        /// Extra flag for layouts to use
        const LAYOUT_3 = 0b0000_0010_0000_0000_0000_0000_0000_0000;
        /// Extra flag for layouts to use
        const LAYOUT_4 = 0b0000_0001_0000_0000_0000_0000_0000_0000;
        /// All extra flags for layouts
        const LAYOUT_ALL   = Self::LAYOUT_1.bits | Self::LAYOUT_2.bits | Self::LAYOUT_3.bits | Self::LAYOUT_4.bits;
        // Extra ones for extensions to use
        /// Extra flag for extensions to use
        const EXT_1 = 0b1000_0000_0000_0000_0000_0000_0000_0000;
        /// Extra flag for extensions to use
        const EXT_2 = 0b0100_0000_0000_0000_0000_0000_0000_0000;
        /// Extra flag for extensions to use
        const EXT_3 = 0b0010_0000_0000_0000_0000_0000_0000_0000;
        /// Extra flag for extensions to use
        const EXT_4 = 0b0001_0000_0000_0000_0000_0000_0000_0000;
        /// All extra flags for extensions
        const EXT_ALL   = Self::EXT_1.bits | Self::EXT_2.bits | Self::EXT_3.bits | Self::EXT_4.bits;
    }
}

/// Extensions extend stylish to allow custom style properties to be added
pub trait Extension {
    /// The type of the data that will be stored on every node
    ///
    /// Can be acccessed via the `.ext` field on `NodeInner`
    type NodeData: Sized;
    /// The type of the extra Values that will be used to extend
    /// the fungui `Value` in `ExtValue` type.
    ///
    /// This is normally an enum
    type Value: Clone + PartialEq + Sized;

    /// Creates a new empty `NodeData` to be stored on a Node.
    fn new_data() -> Self::NodeData;

    /// Called to add new style keys that can be used by style rules
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

    /// Called to apply a given style rule on a node
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
    fn update_data(styles: &Styles<Self>, nc: &NodeChain<Self>, rule: &Rule<Self>, data: &mut Self::NodeData) -> DirtyFlags
        where Self: Sized;

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
    fn reset_unset_data(used_keys: &FnvHashSet<StaticKey>, data: &mut Self::NodeData) -> DirtyFlags;

    /// Called with the flags of a node to allow the data to be updated
    /// based on the dirty state of the node.
    ///
    /// This is useful to marking a node as needing a redraw when it
    /// moves.
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

    /// Add a function that can be called by style rules
    ///
    /// Arguments are only parsed when obtained from the iterator
    /// making unused parameters cheap.
    pub fn add_func_raw<F>(&mut self, name: &'static str, func: F)
    where
        F: for<'a> Fn(&mut (Iterator<Item=FResult<'a, Value<E>>> + 'a)) -> FResult<'a, Value<E>> + 'static,
    {
        let key = self.styles.static_keys.entry(name).or_insert(StaticKey(name));
        self.styles.funcs.insert(*key, Box::new(func));
    }

    /// Adds the node to the root node of this manager.
    ///
    /// The node is created from the passed string.
    /// See [`from_str`](struct.Node.html#from_str)
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
    /// The name can be used to remove the loaded styles later
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
    ///
    /// This will update nodes based on their properties and then
    /// position them based on their selected layout.
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

    /// Renders the nodes in this manager by passing the draw position/size
    /// and style properties to the visitor
    pub fn render<V>(&mut self, visitor: &mut V)
    where
        V: RenderVisitor<E>,
    {
        self.root.render(visitor);
    }
}

/// The position and size of an node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    /// The x position of the node
    pub x: i32,
    /// The y position of the node
    pub y: i32,
    /// The width of the node
    pub width: i32,
    /// The height of the node
    pub height: i32,
}

/// Called for every node in a manager to allow them to
/// be rendered.
pub trait RenderVisitor<E: Extension> {
    /// Called per node before visiting their children
    fn visit(&mut self, node: &mut NodeInner<E>);
    /// Called per node after visiting their children
    fn visit_end(&mut self, node: &mut NodeInner<E>);
}

/// A node representing an element or text.
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

/// Tries to find and evalulate a given style property in a rule.
///
/// This will skip properties that have already been set previously
/// in the update. Should only be used during an `update_(child_)data`
/// call.
///
/// ```ignore
/// eval!(styles, nc, rule.MY_PROP => val => {
///     // This will only run if MY_PROP was set in the rule
///     // val will be a `Value` containing what the property
///     // was set too
/// });
/// ```
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

    /// Returns an immutable reference to the
    /// node's inner value
    #[inline]
    pub fn borrow(&self) -> Ref<NodeInner<E>> {
        self.inner.borrow()
    }

    /// Returns an mutable reference to the
    /// node's inner value
    #[inline]
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
    #[inline]
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
    #[inline]
    pub fn name(&self) -> Option<String> {
        let inner = self.inner.borrow();
        match inner.value {
            NodeValue::Element(ref e) => Some(e.name.clone()),
            NodeValue::Text(_) => None,
        }
    }

    /// Returns whether the passed node points to the same node
    /// as this one
    #[inline]
    pub fn is_same(&self, other: &Node<E>) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }

    /// Returns the text of the node if it is a text node.
    #[inline]
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

    /// Returns a copy of the value for the given property
    /// if it exists.
    #[inline]
    pub fn get_property<V>(&self, key: &str) -> Option<V>
        where V: ConvertValue<E>
    {
        let inner = self.inner.borrow();
        inner.get_property::<V>(key)
    }

    /// Returns a reference to the value for the given property
    /// if it exists.
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

    /// Sets the value of a given property
    #[inline]
    pub fn set_property<V>(&self, key: &str, v: V)
        where V: ConvertValue<E>
    {
        let mut inner = self.inner.borrow_mut();
        inner.properties_changed = true;
        inner.properties.insert(key.into(), V::to_value(v));
    }

    /// Sets the value of a given property without flagging
    /// the node as changed.
    ///
    /// This is useful for when properties are use as storage
    /// and not used in style rules.
    ///
    /// As a general convention this properties should use keys
    /// begining with `$` (e.g. `$cycle`) as these are not accepted
    /// by the style parser.
    #[inline]
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

/// The inner data of a single node.
///
/// `Node` is a wrapper around this to allow it to be passed
/// around easily via reference counting.
pub struct NodeInner<E: Extension> {
    parent: Option<Weak<RefCell<NodeInner<E>>>>,
    properties: FnvHashMap<String, Value<E>>,
    properties_changed: bool,
    possible_rules: Vec<Rc<Rule<E>>>,
    done_layout: bool,
    // Set when added/removed from a node
    rules_dirty: bool,
    dirty_flags: DirtyFlags,
    /// The value of the node.
    ///
    /// The value is either the name and children of
    /// the node or the text of the node.
    pub value: NodeValue<E>,
    /// Whether the text of this node has changed since
    /// last viewed.
    ///
    /// The render visitor should reset this flag after viewing it
    pub text_changed: bool,
    layout: Box<dyn BoxLayoutEngine<E>>,
    parent_data: Box<dyn Any>,
    uses_parent_size: bool,
    prev_rect: Rect,
    /// The current draw position of this node
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

    /// Returns a copy of the value for the given property
    /// if it exists.
    #[inline]
    pub fn get_property<V>(&self, key: &str) -> Option<V>
        where V: ConvertValue<E>
    {
        Self::get_property_impl::<V>(&self.properties, key)
    }

    #[inline]
    fn get_property_ref_impl<'a, V>(props: &'a FnvHashMap<String, Value<E>>, key: &str) -> Option<&'a V::RefType>
        where V: ConvertValue<E>
    {
        props.get(key)
            .and_then(|v| V::from_value_ref(v))
    }

    /// Returns a reference to the value for the given property
    /// if it exists.
    #[inline]
    pub fn get_property_ref<V>(&self, key: &str) -> Option<&V::RefType>
        where V: ConvertValue<E>
    {
        Self::get_property_ref_impl::<V>(&self.properties, key)
    }

    /// Returns the text of the node if it is a text node.
    pub fn text(&self) -> Option<&str> {
        match self.value {
            NodeValue::Element(_) => None,
            NodeValue::Text(ref t) => Some(t.as_str()),
        }
    }
}

/// The value of a node.
///
/// Either an element with children or
/// text node.
pub enum NodeValue<E: Extension> {
    /// An element node, with a name and children
    Element(Element<E>),
    /// A text node
    Text(String),
}

impl <E: Extension> NodeValue<E> {

    /// Returns the text of the node if it is a text node.
    pub fn text(&self) -> Option<&str> {
        match self {
            NodeValue::Element(_) => None,
            NodeValue::Text(ref t) => Some(t.as_str()),
        }
    }
}

/// An element node
pub struct Element<E: Extension> {
    name: String,
    children: Vec<Node<E>>,
}

/// A chain of nodes and their parents
///
/// Used during applying rules for quick traversal.
pub struct NodeChain<'a, E: Extension + 'a> {
    parent: Option<&'a NodeChain<'a, E>>,
    value: NCValue<'a>,
    draw_rect: Rect,
    properties: &'a FnvHashMap<String, Value<E>>,
}

impl <'a, E> NodeChain<'a, E>
    where E: Extension
{
    /// Returns the text of the node if it is a text node.
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

/// A value that can be used as a style property
#[derive(Debug)]
pub enum Value<E: Extension> {
    /// A boolean value
    Boolean(bool),
    /// An integer value
    Integer(i32),
    /// A floating point value
    Float(f64),
    /// A string value
    String(String),
    /// An extension defined value
    ExtValue(E::Value),
}

impl <E> Value<E>
    where E: Extension
{
    /// Attemps to convert this value into the given
    /// type
    pub fn convert<V>(self) -> Option<V>
        where V: ConvertValue<E>
    {
        V::from_value(self)
    }

    /// Attemps to convert a reference to this value into
    /// the given reference type
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

/// Types that can be converted to and from a value
pub trait ConvertValue<E: Extension>: Sized {
    /// The reference type of this value.
    ///
    /// Useful for types like `String` where the reference
    /// type is `str`
    type RefType: ?Sized;

    /// Tries to convert from the passed value to this type
    fn from_value(v: Value<E>) -> Option<Self>;
    /// Tries to convert from the passed value to the reference
    /// type.
    fn from_value_ref(v: &Value<E>) -> Option<&Self::RefType>;
    /// Converts the value into a `Value`
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