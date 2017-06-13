extern crate stylish_syntax as syntax;
#[macro_use]
extern crate error_chain;

pub mod query;
pub mod error;
mod rule;
use rule::*;

/// The error type used by stylish
pub type SResult<T> = error::Result<T>;
use error::{ErrorKind, ResultExt};

use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::collections::HashMap;
use std::any::Any;

/// Stores loaded nodes and manages the layout.
pub struct Manager<RInfo> {
    // Has no parent, is the parent for all base nodes
    // in the system
    root: Node<RInfo>,
    styles: Styles<RInfo>,
    last_size: (i32, i32),
}

impl <RInfo> Manager<RInfo> {
    /// Creates a new manager with an empty root node.
    pub fn new() -> Manager<RInfo> {
        Manager {
            root: Node::root(),
            styles: Styles {
                styles: Vec::new(),
                layouts: {
                    let mut layouts: HashMap<String, Box<Fn(&RenderObject<RInfo>) -> Box<LayoutEngine<RInfo>>>> = HashMap::new();
                    layouts.insert("absolute".to_owned(), Box::new(|_| Box::new(AbsoluteLayout)));
                    layouts
                },
                funcs: HashMap::new(),
            },
            last_size: (0, 0),
        }
    }

    /// Adds a new function that can be used to create a layout engine.
    ///
    /// A layout engine is used to position elements within an element.
    ///
    /// The layout engine can be selected by using the `layout` attribute.
    pub fn add_layout_engine<F>(&mut self, name: &str, creator: F)
        where F: Fn(&RenderObject<RInfo>) -> Box<LayoutEngine<RInfo>> + 'static
    {
        self.styles.layouts.insert(name.into(), Box::new(creator));
    }

    /// Add a function that can be called by styles
    pub fn add_func_raw<F>(&mut self, name: &str, func: F)
        where F: Fn(Vec<Value>) -> SResult<Value> + 'static
    {
        self.styles.funcs.insert(name.into(), Box::new(func));
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
    pub fn add_node(&mut self, node: Node<RInfo>) {
        assert!(node.inner.borrow().parent.is_none(), "Node already has a parent");
        if let NodeValue::Element(ref mut e) = self.root.inner.borrow_mut().value {
            node.inner.borrow_mut().parent = Some(Rc::downgrade(&self.root.inner));
            e.children.push(node);
        } else {
            panic!("Text cannot have child elements")
        }
    }

    /// Starts a query from the root of this manager
    pub fn query(&self) -> query::Query<RInfo> {
        query::Query::new(self.root.clone())
    }

    /// Loads a set of styles from the given string.
    pub fn load_styles<'a>(&mut self, name: &str, style_rules: &'a str) -> Result<(), syntax::PError<'a>> {
        let styles = syntax::style::Document::parse(style_rules)?;
        self.styles.styles.push((name.into(), styles));
        Ok(())
    }

    /// Renders the nodes in this manager by passing the
    /// layout and styles to the passed visitor.
    pub fn render<V>(&mut self, visitor: &mut V, width: i32, height: i32)
        where V: RenderVisitor<RInfo>
    {
        let dirty = self.last_size != (width, height);
        self.last_size = (width, height);
        let screen = Rect {
            x: 0, y: 0,
            width: width,
            height: height
        };
        self.root.set_property("width", width);
        self.root.set_property("height", height);
        let inner = self.root.inner.borrow();
        if let NodeValue::Element(ref e) = inner.value {
            for c in &e.children {
                c.render(&self.styles, &mut AbsoluteLayout, visitor, screen, dirty);
            }
        }
    }
}

/// Used to position an element within another element.
pub trait LayoutEngine<RInfo> {
    /// Called when the element needs to be positioned. Should
    /// set the value of `draw_rect` on the passed object.
    fn position_element(&mut self, obj: &mut RenderObject<RInfo>);
}

impl <RInfo> LayoutEngine<RInfo> for Box<LayoutEngine<RInfo>> {
    fn position_element(&mut self, obj: &mut RenderObject<RInfo>) {
        (**self).position_element(obj)
    }
}

/// The default layout.
///
/// Copies the values of `x`, `y`, `width` and `height` directly
/// to the element's layout.
struct AbsoluteLayout;

impl <RInfo> LayoutEngine<RInfo> for AbsoluteLayout {
    fn position_element(&mut self, obj: &mut RenderObject<RInfo>) {
        obj.draw_rect = Rect {
            x: obj.get_value::<i32>("x").unwrap_or(0),
            y: obj.get_value::<i32>("y").unwrap_or(0),
            width: obj.get_value::<i32>("width").unwrap_or(0),
            height: obj.get_value::<i32>("height").unwrap_or(0),
        }
    }
}

/// The position and size of an element
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Called for every element in a manager to allow them to
/// be rendered.
pub trait RenderVisitor<RInfo> {
    /// Called with an element to be rendered.
    fn visit(&mut self, obj: &mut RenderObject<RInfo>);
    /// Called after all of the passed element's children
    /// have been visited.
    fn visit_end(&mut self, _obj: &mut RenderObject<RInfo>) {}
}

struct Styles<RInfo> {
    styles: Vec<(String, syntax::style::Document)>,
    layouts: HashMap<String, Box<Fn(&RenderObject<RInfo>) -> Box<LayoutEngine<RInfo>>>>,
    funcs: HashMap<String, Box<Fn(Vec<Value>) -> SResult<Value>>>,
}

impl <RInfo> Styles<RInfo> {
    // TODO: Remove boxing
    fn find_matching_rules<'a, 'b>(&'a self, node: &'b Node<RInfo>) -> RuleIter<'b, Box<Iterator<Item=&'a syntax::style::Rule> +'a>, RInfo> {
        let iter = self.styles.iter()
            .map(|v| &v.1)
            .flat_map(|v| &v.rules)
            .rev();
        RuleIter {
            node: node,
            rules: Box::new(iter) as _,
        }
    }
}

/// A node representing an element.
///
/// Can be cloned to duplicate the reference to the node.
pub struct Node<RInfo> {
    inner: Rc<RefCell<NodeInner<RInfo>>>,
}

impl <RInfo> Clone for Node<RInfo> {
    fn clone(&self) -> Self {
        Node {
            inner: self.inner.clone(),
        }
    }
}

impl <RInfo> Node<RInfo> {
    fn render<V, L>(&self, styles: &Styles<RInfo>, layout: &mut L, visitor: &mut V, area: Rect, force_dirty: bool)
        where V: RenderVisitor<RInfo>,
              L: LayoutEngine<RInfo>,
    {
        use std::collections::hash_map::Entry;
        let mut dirty = force_dirty;
        {
            let missing_obj = {
                self.inner.borrow()
                    .render_object
                    .is_none()
            };
            if missing_obj || force_dirty {
                dirty = true;
                let mut obj = RenderObject::default();
                for rule in styles.find_matching_rules(self) {
                    for key in rule.syn.styles.keys() {
                        let key = &key.name;
                        if let Entry::Vacant(e) = obj.vars.entry(key.clone()) {
                            if let Some(v) = rule.get_value(styles, key) {
                                e.insert(v);
                            }
                        }
                    }
                }
                layout.position_element(&mut obj);
                if let Some(layout) = obj.get_value::<String>("layout") {
                    if let Some(engine) = styles.layouts.get(&layout) {
                        obj.layout_engine = RefCell::new(engine(&obj));
                    }
                }
                let mut inner = self.inner.borrow_mut();
                if let NodeValue::Text(ref txt) = inner.value {
                    obj.text = Some(txt.clone());
                }
                inner.render_object = Some(obj);
            }
            let mut inner = self.inner.borrow_mut();
            if let Some(render) = inner.render_object.as_mut() {
                visitor.visit(render);
            }
        }
        {
            let inner = self.inner.borrow();
            if let Some(render) = inner.render_object.as_ref() {
                let mut layout_engine = render.layout_engine.borrow_mut();
                if let NodeValue::Element(ref e) = inner.value {
                    for c in &e.children {
                        c.render(styles, &mut *layout_engine, visitor, render.draw_rect, dirty);
                    }
                }
            }
        }

        let mut inner = self.inner.borrow_mut();
        if let Some(render) = inner.render_object.as_mut() {
            visitor.visit_end(render);
        }
    }

    /// Adds the passed node as a child to this node.
    ///
    /// This panics if the passed node already has a parent
    /// or if the node is a text node.
    pub fn add_child(&self, node: Node<RInfo>) {
        assert!(node.inner.borrow().parent.is_none(), "Node already has a parent");
        if let NodeValue::Element(ref mut e) = self.inner.borrow_mut().value {
            node.inner.borrow_mut().parent = Some(Rc::downgrade(&self.inner));
            e.children.push(node);
        } else {
            panic!("Text cannot have child elements")
        }
    }

    /// Returns the name of the node if it has one
    pub fn name(&self) -> Option<String> {
        let inner = self.inner.borrow();
        match inner.value {
            NodeValue::Element(ref e) => Some(e.name.clone()),
            NodeValue::Text(_) => None,
        }
    }

    /// Returns the value of the property if it has it set
    pub fn get_property<V: PropertyValue>(&self, key: &str) -> Option<V> {
        let inner = self.inner.borrow();
        match inner.value {
            NodeValue::Element(ref e) => e.properties.get(key).and_then(|v| V::convert_from(&v)),
            NodeValue::Text(_) => None,
        }
    }

    /// Sets the value of the property on the node.
    ///
    /// Only valid on non-text nodes.
    pub fn set_property<V: PropertyValue>(&self, key: &str, value: V){
        let mut inner = self.inner.borrow_mut();
        inner.render_object = None;
        match inner.value {
            NodeValue::Element(ref mut e) => {e.properties.insert(key.into(), value.convert_into());},
            NodeValue::Text(_) => {},
        }
    }

    /// Begins a query on this node
    pub fn query(&self) -> query::Query<RInfo> {
        query::Query::new(self.clone())
    }

    /// Creates a node from a string
    pub fn from_str(s: &str) -> Result<Node<RInfo>, syntax::PError> {
        syntax::desc::Document::parse(s)
            .map(|v| Node::from_document(v))
    }

    /// Creates a node from a parsed document.
    pub fn from_document(desc: syntax::desc::Document) -> Node<RInfo> {
        Node::from_doc_element(desc.root)
    }

    fn from_doc_text(desc: String) -> Node<RInfo> {
        Node {
            inner: Rc::new(RefCell::new(NodeInner {
                parent: None,
                value: NodeValue::Text(desc),
                render_object: None,
            }))
        }
    }

    fn from_doc_element(desc: syntax::desc::Element) -> Node<RInfo> {
        let node = Node {
            inner: Rc::new(RefCell::new(NodeInner {
                parent: None,
                value: NodeValue::Element(Element {
                    name: desc.name.name,
                    children: Vec::with_capacity(desc.nodes.len()),
                    properties: desc.properties.into_iter()
                        .map(|(n, v)| (n.name, v.into()))
                        .collect()
                }),
                render_object: None,
            }))
        };

        for c in desc.nodes.into_iter()
            .map(|n| match n {
                syntax::desc::Node::Element(e) => Node::from_doc_element(e),
                syntax::desc::Node::Text(t, _) => Node::from_doc_text(t),
            })
        {
            node.add_child(c);
        }

        node
    }

    fn root() -> Node<RInfo> {
        Node {
            inner: Rc::new(RefCell::new(NodeInner {
                parent: None,
                value: NodeValue::Element(Element {
                    name: "root".into(),
                    properties: HashMap::default(),
                    children: Vec::new(),
                }),
                render_object: None,
            })),
        }
    }
}

struct NodeInner<RInfo> {
    parent: Option<Weak<RefCell<NodeInner<RInfo>>>>,
    value: NodeValue<RInfo>,
    render_object: Option<RenderObject<RInfo>>,
}

enum NodeValue<RInfo> {
    Element(Element<RInfo>),
    Text(String)
}

struct Element<RInfo> {
    name: String,
    properties: HashMap<String, Value>,
    children: Vec<Node<RInfo>>,
}

/// A value that can be used as a style attribute
#[derive(Debug)]
pub enum Value {
    Integer(i32),
    Float(f64),
    String(String),
    Any(Box<CustomValue>),
}

impl Value {
    /// Tries to convert this value into the type.
    pub fn get_value<V: PropertyValue>(&self) -> Option<V> {
         V::convert_from(self)
    }

    /// Tries to convert this value into the custom type.
    pub fn get_custom_value<V: CustomValue + 'static>(&self) -> Option<&V> {
        if let Value::Any(ref v) = *self {
            (**v).as_any().downcast_ref::<V>()
        } else {
            None
        }
    }
}

impl Clone for Value {
    fn clone(&self) -> Value {
        match *self {
            Value::Integer(v) => Value::Integer(v),
            Value::Float(v) => Value::Float(v),
            Value::String(ref v) => Value::String(v.clone()),
            Value::Any(ref v) => Value::Any((*v).clone()),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, rhs: &Value) -> bool {
        use Value::*;
        match (self, rhs) {
            (&Integer(a), &Integer(b)) => a == b,
            (&Float(a), &Float(b)) => a == b,
            (&String(ref a), &String(ref b)) => a == b,
            _ => false,
        }
    }
}

impl From<syntax::desc::ValueType> for Value {
    fn from(v: syntax::desc::ValueType) -> Value {
        match v.value {
            syntax::desc::Value::Integer(val) => Value::Integer(val),
            syntax::desc::Value::Float(val) => Value::Float(val),
            syntax::desc::Value::String(val) => Value::String(val),
        }
    }
}

/// The value passed to layout engines and render visitors
/// in order to render the nodes.
///
/// `render_info` is used by the renderer and not stylish.
pub struct RenderObject<RInfo> {
    /// The position and size of the element
    /// as decided by the layout engine.
    pub draw_rect: Rect,
    layout_engine: RefCell<Box<LayoutEngine<RInfo>>>,
    vars: HashMap<String, Value>,
    /// Renderer storage
    pub render_info: Option<RInfo>,
    /// The text of this element if it is text.
    pub text: Option<String>,
}

impl <RInfo> RenderObject<RInfo> {
    /// Gets the value from the style rules for this element
    pub fn get_value<V: PropertyValue>(&self, name: &str) -> Option<V> {
        self.vars.get(name)
            .and_then(|v| V::convert_from(&v))
    }

    /// Gets the custom value from the style rules for this element
    pub fn get_custom_value<V: CustomValue + 'static>(&self, name: &str) -> Option<&V> {
        self.vars.get(name)
            .and_then(|v| if let Value::Any(ref v) = *v {
                (**v).as_any().downcast_ref::<V>()
            } else { None })
    }
}

impl <RInfo> Default for RenderObject<RInfo> {
    fn default() -> RenderObject<RInfo> {
        RenderObject {
            draw_rect: Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            layout_engine: RefCell::new(Box::new(AbsoluteLayout)),
            vars: HashMap::new(),
            render_info: Default::default(),
            text: None,
        }
    }
}

/// A value that can be stored as a property
pub trait PropertyValue: Sized {
    /// Converts a value into this type
    fn convert_from(v: &Value) -> Option<Self>;
    /// Converts this type into a value
    fn convert_into(self) -> Value;
}

/// A type that can be converted into `Any`
pub trait Anyable: Any {
    /// Converts this type to `Any`
    fn as_any(&self) -> &Any;
}

impl <T: Any> Anyable for T {
    fn as_any(&self) -> &Any {
        self
    }
}

/// A non-standard type that can be used as a property
/// value.
pub trait CustomValue: Anyable {
    /// Clones this type
    fn clone(&self) -> Box<CustomValue>;
}

impl ::std::fmt::Debug for Box<CustomValue> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "CustomValue")
    }
}

impl <T: Clone + 'static> CustomValue for Vec<T> {
    fn clone(&self) -> Box<CustomValue> {
        Box::new(Clone::clone(self))
    }
}

impl <T: CustomValue + 'static> PropertyValue for T {
    fn convert_from(_v: &Value) -> Option<Self> {
        panic!("Can't convert into T")
    }
    fn convert_into(self) -> Value {
        Value::Any(Box::new(self))
    }
}

impl PropertyValue for Value {
    fn convert_from(v: &Value) -> Option<Self> { Some(v.clone()) }
    fn convert_into(self) -> Value { self }
}

impl PropertyValue for i32 {
    fn convert_from(v: &Value) -> Option<Self> {
        match *v {
            Value::Integer(v) => Some(v),
            _ => None,
        }
    }

    fn convert_into(self) -> Value {
        Value::Integer(self)
    }
}

impl PropertyValue for f64 {
    fn convert_from(v: &Value) -> Option<Self> {
        match *v {
            Value::Float(v) => Some(v),
            _ => None,
        }
    }

    fn convert_into(self) -> Value {
        Value::Float(self)
    }
}

impl PropertyValue for String {
    fn convert_from(v: &Value) -> Option<Self> {
        match *v {
            Value::String(ref v) => Some(v.clone()),
            _ => None,
        }
    }

    fn convert_into(self) -> Value {
        Value::String(self)
    }
}
