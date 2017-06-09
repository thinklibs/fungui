extern crate stylish_syntax as syntax;

pub mod query;

use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::collections::HashMap;

pub struct Manager<RInfo> {
    // Has no parent, is the parent for all base nodes
    // in the system
    root: Node<RInfo>,

    styles: Styles<RInfo>,
}

impl <RInfo> Manager<RInfo> {
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
            },
        }
    }

    pub fn add_layout_engine<F>(&mut self, name: &str, creator: F)
        where F: Fn(&RenderObject<RInfo>) -> Box<LayoutEngine<RInfo>> + 'static
    {
        self.styles.layouts.insert(name.into(), Box::new(creator));
    }

    pub fn add_node(&mut self, node: Node<RInfo>) {
        assert!(node.inner.borrow().parent.is_none(), "Node already has a parent");
        if let NodeValue::Element(ref mut e) = self.root.inner.borrow_mut().value {
            node.inner.borrow_mut().parent = Some(Rc::downgrade(&self.root.inner));
            e.children.push(node);
        } else {
            panic!("Text cannot have child elements")
        }
    }

    pub fn query(&self) -> query::Query<RInfo> {
        query::Query::new(self.root.clone())
    }

    pub fn load_styles<'a>(&mut self, name: &str, style_rules: &'a str) -> Result<(), syntax::PError<'a>> {
        let styles = syntax::style::Document::parse(style_rules)?;
        self.styles.styles.push((name.into(), styles));
        Ok(())
    }

    pub fn render<V>(&mut self, visitor: &mut V, width: i32, height: i32)
        where V: RenderVisitor<RInfo>
    {
        let screen = Rect {
            x: 0, y: 0,
            width: width,
            height: height
        };
        let inner = self.root.inner.borrow();
        if let NodeValue::Element(ref e) = inner.value {
            for c in &e.children {
                c.render(&self.styles, &mut AbsoluteLayout, visitor, screen, false);  // TODO: Force dirty on resize?
            }
        }
    }
}

pub trait LayoutEngine<RInfo> {
    fn position_element(&mut self, obj: &RenderObject<RInfo>) -> Rect;
}

impl <RInfo> LayoutEngine<RInfo> for Box<LayoutEngine<RInfo>> {
    fn position_element(&mut self, obj: &RenderObject<RInfo>) -> Rect {
        (**self).position_element(obj)
    }
}

struct AbsoluteLayout;

impl <RInfo> LayoutEngine<RInfo> for AbsoluteLayout {
    fn position_element(&mut self, obj: &RenderObject<RInfo>) -> Rect {
        Rect {
            x: obj.get_value::<i32>("x").unwrap_or(0),
            y: obj.get_value::<i32>("y").unwrap_or(0),
            width: obj.get_value::<i32>("width").unwrap_or(0),
            height: obj.get_value::<i32>("height").unwrap_or(0),
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub trait RenderVisitor<RInfo> {
    fn visit(&mut self, obj: &mut RenderObject<RInfo>);
}

struct Styles<RInfo> {
    styles: Vec<(String, syntax::style::Document)>,
    layouts: HashMap<String, Box<Fn(&RenderObject<RInfo>) -> Box<LayoutEngine<RInfo>>>>,
}

impl <RInfo> Styles<RInfo> {
    // TODO: Remove boxing
    fn find_matching_rules<'a, 'b>(&'a self, node: &'b Node<RInfo>) -> RuleIter<'b, Box<Iterator<Item=&'a syntax::style::Rule> +'a>, RInfo> {
        let iter = self.styles.iter()
            .map(|v| &v.1)
            .flat_map(|v| &v.rules);
        RuleIter {
            node: node,
            rules: Box::new(iter) as _,
        }
    }
}

struct RuleIter<'a, I, RInfo: 'a> {
    node: &'a Node<RInfo>,
    rules: I,
}

#[derive(Debug)]
struct Rule<'a> {
    syn: &'a syntax::style::Rule,
    vars: HashMap<String, Value>,
}

impl <'a> Rule<'a> {
    fn eval_value(&self, val: &syntax::style::Value) -> Value {
        use syntax::style;
        match *val {
            style::Value::Float(f) => Value::Float(f),
            style::Value::Integer(i) => Value::Integer(i),
            style::Value::String(ref s) => Value::String(s.clone()),
            style::Value::Variable(ref name) => self.vars.get(&name.name).unwrap().clone(),
        }
    }

    fn eval(&self, expr: &syntax::style::Expr) -> Value {
        use syntax::style;
        match *expr {
            style::Expr::Value(ref v) => self.eval_value(v),
            style::Expr::Add(ref l, ref r) => {
                let l = self.eval(&l.expr);
                let r = self.eval(&r.expr);
                match (l, r) {
                    (Value::Float(l), Value::Float(r)) => Value::Float(l + r),
                    (Value::Integer(l), Value::Integer(r)) => Value::Integer(l + r),
                    (Value::String(l), Value::String(r)) => Value::String(l + &r),
                    _ => panic!("Can't add these types"),
                }
            },
            style::Expr::Sub(ref l, ref r) => {
                let l = self.eval(&l.expr);
                let r = self.eval(&r.expr);
                match (l, r) {
                    (Value::Float(l), Value::Float(r)) => Value::Float(l - r),
                    (Value::Integer(l), Value::Integer(r)) => Value::Integer(l - r),
                    _ => panic!("Can't subtract these types"),
                }
            },
            style::Expr::Mul(ref l, ref r) => {
                let l = self.eval(&l.expr);
                let r = self.eval(&r.expr);
                match (l, r) {
                    (Value::Float(l), Value::Float(r)) => Value::Float(l * r),
                    (Value::Integer(l), Value::Integer(r)) => Value::Integer(l * r),
                    _ => panic!("Can't multiply these types"),
                }
            },
            style::Expr::Div(ref l, ref r) => {
                let l = self.eval(&l.expr);
                let r = self.eval(&r.expr);
                match (l, r) {
                    (Value::Float(l), Value::Float(r)) => Value::Float(l / r),
                    (Value::Integer(l), Value::Integer(r)) => Value::Integer(l / r),
                    _ => panic!("Can't divide these types"),
                }
            },
            style::Expr::Neg(ref l) => {
                let l = self.eval(&l.expr);
                match l {
                    Value::Float(l) => Value::Float(-l),
                    Value::Integer(l) => Value::Integer(-l),
                    _ => panic!("Can't negative this type"),
                }
            },
            _ => unimplemented!(),
        }
    }

    fn get_value<V: PropertyValue>(&self, name: &str) -> Option<V> {
        use syntax::Ident;
        let ident = Ident {
            name: name.into(),
            .. Default::default()
        };
        if let Some(expr) = self.syn.styles.get(&ident) {
            let val = self.eval(&expr.expr);
            V::convert_from(val)
        } else {
            None
        }
    }
}

impl <'a, 'b, I, RInfo> Iterator for RuleIter<'b, I, RInfo>
    where I: Iterator<Item=&'a syntax::style::Rule> + 'a
{
    type Item = Rule<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        use syntax::style;
        'search:
        while let Some(rule) = self.rules.next() {
            let mut current = Some(self.node.clone());
            let mut vars: HashMap<String, Value> = HashMap::new();
            for m in rule.matchers.iter().rev() {
                if let Some(cur) = current.take() {
                    let cur = cur.inner.borrow();
                    match (m, &cur.value) {
                        (&style::Matcher::Text, &NodeValue::Text(..)) => {},
                        (&style::Matcher::Element(ref e), &NodeValue::Element(ref ne)) => {
                            if e.name.name != ne.name {
                                continue 'search;
                            }
                            for (prop, v) in &e.properties {
                                if let Some(nprop) = ne.properties.get(&prop.name) {
                                    match (&v.value, nprop) {
                                        (
                                            &style::Value::Variable(ref name),
                                            val
                                        ) => {
                                            vars.insert(name.name.clone(), val.clone());
                                        },
                                        (
                                            &style::Value::Integer(i),
                                            &Value::Integer(ni),
                                        ) if ni == i => {},
                                        (
                                            &style::Value::Float(f),
                                            &Value::Float(nf),
                                        ) if nf == f => {},
                                        (
                                            &style::Value::String(ref s),
                                            &Value::String(ref ns),
                                        ) if ns == s => {},
                                        _ => continue 'search,
                                    }
                                } else {
                                    continue 'search;
                                }
                            }
                        },
                        _ => continue 'search,
                    }
                    current = cur.parent.as_ref()
                        .and_then(|v| v.upgrade())
                        .map(|v| Node { inner: v });
                } else {
                    continue 'search;
                }
            }
            return Some(Rule {
                syn: rule,
                vars: vars,
            });
        }
        None
    }
}

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
                        rule.get_value(key).map(|v| obj.vars.insert(key.clone(), v));
                    }
                }
                obj.draw_rect = layout.position_element(&obj);
                obj.draw_rect.x += area.x;
                obj.draw_rect.y += area.y;
                if let Some(layout) = obj.get_value::<String>("layout") {
                    if let Some(engine) = styles.layouts.get(&layout) {
                        obj.layout_engine = RefCell::new(engine(&obj));
                    }
                }
                let mut inner = self.inner.borrow_mut();
                inner.render_object = Some(obj);
            }
            let mut inner = self.inner.borrow_mut();
            if let Some(render) = inner.render_object.as_mut() {
                visitor.visit(render);
            }
        }
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

    pub fn add_child(&self, node: Node<RInfo>) {
        assert!(node.inner.borrow().parent.is_none(), "Node already has a parent");
        if let NodeValue::Element(ref mut e) = self.inner.borrow_mut().value {
            node.inner.borrow_mut().parent = Some(Rc::downgrade(&self.inner));
            e.children.push(node);
        } else {
            panic!("Text cannot have child elements")
        }
    }

    pub fn name(&self) -> Option<String> {
        let inner = self.inner.borrow();
        match inner.value {
            NodeValue::Element(ref e) => Some(e.name.clone()),
            NodeValue::Text(_) => None,
        }
    }

    pub fn get_property<V: PropertyValue>(&self, key: &str) -> Option<V> {
        let inner = self.inner.borrow();
        match inner.value {
            NodeValue::Element(ref e) => e.properties.get(key).and_then(|v| V::convert_from(v.clone())),
            NodeValue::Text(_) => None,
        }
    }

    pub fn set_property<V: PropertyValue>(&self, key: &str, value: V){
        let mut inner = self.inner.borrow_mut();
        inner.render_object = None;
        match inner.value {
            NodeValue::Element(ref mut e) => {e.properties.insert(key.into(), value.convert_into());},
            NodeValue::Text(_) => {},
        }
    }

    pub fn query(&self) -> query::Query<RInfo> {
        query::Query::new(self.clone())
    }

    pub fn from_str(s: &str) -> Result<Node<RInfo>, syntax::PError> {
        syntax::desc::Document::parse(s)
            .map(|v| Node::from_document(v))
    }

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

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i32),
    Float(f64),
    String(String),
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

pub struct RenderObject<RInfo> {
    pub draw_rect: Rect,
    layout_engine: RefCell<Box<LayoutEngine<RInfo>>>,
    vars: HashMap<String, Value>,
    pub render_info: Option<RInfo>,
}

impl <RInfo> RenderObject<RInfo> {
    pub fn get_value<V: PropertyValue>(&self, name: &str) -> Option<V> {
        self.vars.get(name)
            .and_then(|v| V::convert_from(v.clone()))
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
        }
    }
}

pub trait PropertyValue: Sized {
    fn convert_from(v: Value) -> Option<Self>;
    fn convert_into(self) -> Value;
}

impl PropertyValue for Value {
    fn convert_from(v: Value) -> Option<Self> { Some(v) }
    fn convert_into(self) -> Value { self }
}

impl PropertyValue for i32 {
    fn convert_from(v: Value) -> Option<Self> {
        match v {
            Value::Integer(v) => Some(v),
            _ => None,
        }
    }

    fn convert_into(self) -> Value {
        Value::Integer(self)
    }
}

impl PropertyValue for f64 {
    fn convert_from(v: Value) -> Option<Self> {
        match v {
            Value::Float(v) => Some(v),
            _ => None,
        }
    }

    fn convert_into(self) -> Value {
        Value::Float(self)
    }
}

impl PropertyValue for String {
    fn convert_from(v: Value) -> Option<Self> {
        match v {
            Value::String(v) => Some(v),
            _ => None,
        }
    }

    fn convert_into(self) -> Value {
        Value::String(self)
    }
}
