extern crate stylish_syntax as syntax;

pub mod query;

use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::collections::HashMap;

pub struct Manager {
    // Has no parent, is the parent for all base nodes
    // in the system
    root: Node,
}

impl Manager {
    pub fn new() -> Manager {
        Manager {
            root: Node::root(),
        }
    }

    pub fn add_node(&mut self, node: Node) {
        assert!(node.inner.borrow().parent.is_none(), "Node already has a parent");
        if let NodeValue::Element(ref mut e) = self.root.inner.borrow_mut().value {
            node.inner.borrow_mut().parent = Some(Rc::downgrade(&self.root.inner));
            e.children.push(node);
        } else {
            panic!("Text cannot have child elements")
        }
    }

    pub fn render<V>(&mut self, visitor: &mut V)
        where V: RenderVisitor
    {
        self.root.render(visitor, false); // TODO: Force dirty on resize?
    }

}

pub trait RenderVisitor {
    fn visit(&mut self, obj: &RenderObject);
}

#[derive(Clone)]
pub struct Node {
    inner: Rc<RefCell<NodeInner>>,
}

impl Node {

    fn render<V>(&self, visitor: &mut V, force_dirty: bool)
        where V: RenderVisitor
    {
        let mut dirty = force_dirty;
        {
            let mut inner = self.inner.borrow_mut();
            if inner.render_object.is_none() || force_dirty {
                dirty = true;
                inner.render_object = Some(RenderObject {
                    name: if let NodeValue::Element(ref e) = inner.value {
                        e.name.clone()
                    } else {
                        "$text$".into()
                    },
                });
            }
            if let Some(render) = inner.render_object.as_ref() {
                visitor.visit(&render);
            }
        }
        let inner = self.inner.borrow();
        if let NodeValue::Element(ref e) = inner.value {
            for c in &e.children {
                c.render(visitor, dirty);
            }
        }
    }

    pub fn add_child(&self, node: Node) {
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

    pub fn query(&self) -> query::Query {
        query::Query::new(self.clone())
    }

    pub fn from_document(desc: syntax::desc::Document) -> Node {
        Node::from_doc_element(desc.root)
    }

    fn from_doc_text(desc: String) -> Node {
        Node {
            inner: Rc::new(RefCell::new(NodeInner {
                parent: None,
                value: NodeValue::Text(desc),
                render_object: None,
            }))
        }
    }

    fn from_doc_element(desc: syntax::desc::Element) -> Node {

        let node = Node {
            inner: Rc::new(RefCell::new(NodeInner {
                parent: None,
                value: NodeValue::Element(Element {
                    name: desc.name.name,
                    children: Vec::with_capacity(desc.nodes.len()),
                    properties: desc.properties.into_iter()
                        .map(|(n, v)| (n.name, match v.value {
                            syntax::desc::Value::Integer(val) => Value::Integer(val),
                            syntax::desc::Value::Float(val) => Value::Float(val),
                            syntax::desc::Value::String(val) => Value::String(val),
                        }))
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

    fn root() -> Node {
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

struct NodeInner {
    parent: Option<Weak<RefCell<NodeInner>>>,
    value: NodeValue,
    render_object: Option<RenderObject>,
}

enum NodeValue {
    Element(Element),
    Text(String)
}

struct Element {
    name: String,
    properties: HashMap<String, Value>,
    children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i32),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone)]
pub struct RenderObject {
    name: String, // TODO: TMP
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


// #[test]
// fn test() {
//     let doc = syntax::desc::Document::parse(r#"
// panel {
//     icon(type="warning")
//     "testing"
// }

// "#).unwrap();
//     let node = Node::from_document(doc);
//     let mut manager = Manager::new();
//     manager.add_node(node);

//     struct Printer;
//     impl RenderVisitor for Printer {
//         fn visit(&mut self, obj: &RenderObject) {
//             println!("{:?}", obj);
//         }
//     }
//     manager.render(&mut Printer);
//     panic!("TODO");
// }