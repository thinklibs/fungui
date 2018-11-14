
use super::*;
use std::borrow::Cow;

pub struct Query<'a, E: Extension + 'a> {
    pub(crate) root: Node<E>,
    pub(crate) rules: Vec<Rule<'a, E>>,
    pub(crate) location: Option<AtLocation>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AtLocation {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

pub(crate) enum Rule<'a, E: Extension + 'a> {
    /// Matches against child nodes
    Child,
    /// Matches against the element's name
    Name(Cow<'a, str>),
    /// Matches against a property
    Property(Cow<'a, str>, ValueRef<'a, E>),
    /// Matches against a text node
    Text,
}

pub enum ValueRef<'a, E: Extension + 'a> {
    Boolean(bool),
    Integer(i32),
    Float(f64),
    String(Cow<'a, str>),
    ExtValue(&'a E::Value),
}

impl <'a, E> Clone for ValueRef<'a, E>
    where E: Extension + 'a
{
    fn clone(&self) -> Self {
        match self {
            ValueRef::Boolean(v) => ValueRef::Boolean(*v),
            ValueRef::Integer(v) => ValueRef::Integer(*v),
            ValueRef::Float(v) => ValueRef::Float(*v),
            ValueRef::String(v) => ValueRef::String(v.clone()),
            ValueRef::ExtValue(v) => ValueRef::ExtValue(*v),
        }
    }
}

// impl <'a, E> Copy for ValueRef<'a, E>
//     where E: Extension + 'a
// {}

pub trait AsValueRef<'a, E: Extension> {
    fn as_value_ref(self) -> ValueRef<'a, E>;
}

impl <'a, E> AsValueRef<'a, E> for &'a str
    where E: Extension
{
    fn as_value_ref(self) -> ValueRef<'a, E> {
        ValueRef::String(self.into())
    }
}
impl <E> AsValueRef<'static, E> for String
    where E: Extension
{
    fn as_value_ref(self) -> ValueRef<'static, E> {
        ValueRef::String(self.into())
    }
}

impl <'a, E> AsValueRef<'a, E> for ValueRef<'a, E>
    where E: Extension
{
    fn as_value_ref(self) -> ValueRef<'a, E> {
        self
    }
}
impl <'a, E> AsValueRef<'a, E> for i32
    where E: Extension
{
    fn as_value_ref(self) -> ValueRef<'a, E> {
        ValueRef::Integer(self)
    }
}
impl <'a, E> AsValueRef<'a, E> for f64
    where E: Extension
{
    fn as_value_ref(self) -> ValueRef<'a, E> {
        ValueRef::Float(self)
    }
}
impl <'a, E> AsValueRef<'a, E> for f32
    where E: Extension
{
    fn as_value_ref(self) -> ValueRef<'a, E> {
        ValueRef::Float(self as f64)
    }
}
impl <'a, E> AsValueRef<'a, E> for bool
    where E: Extension
{
    fn as_value_ref(self) -> ValueRef<'a, E> {
        ValueRef::Boolean(self)
    }
}

impl<'a, E> Query<'a, E>
    where E: Extension + 'a
{
    pub(super) fn new(node: Node<E>) -> Query<'a, E> {
        Query {
            root: node,
            rules: vec![],
            location: None,
        }
    }

    /// Converts an empty query into an owned one
    pub fn into_owned(self) -> Query<'static, E> {
        assert!(self.rules.is_empty());
        Query {
            root: self.root,
            rules: vec![],
            location: self.location,
        }
    }

    pub fn name<S>(mut self, name: S) -> Query<'a, E>
        where S: Into<Cow<'a, str>>,
    {
        self.rules.push(Rule::Name(name.into()));
        self
    }

    pub fn text(mut self) -> Query<'a, E> {
        self.rules.push(Rule::Text);
        self
    }

    pub fn property<S, V>(mut self, key: S, val: V) -> Query<'a, E>
    where
        V: AsValueRef<'a, E> + 'a,
        S: Into<Cow<'a, str>>,
    {
        self.rules
            .push(Rule::Property(key.into(), val.as_value_ref()));
        self
    }

    pub fn child(mut self) -> Query<'a, E> {
        self.rules.push(Rule::Child);
        self
    }

    pub fn matches(self) -> QueryIterator<'a, E> {
        let rect = if let Some(loc) = self.location {
            let rect = self.root.render_position().unwrap_or(Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            });

            if loc.x < rect.x || loc.x >= rect.x + rect.width || loc.y < rect.y
                || loc.y >= rect.y + rect.height
            {
                return QueryIterator {
                    nodes: vec![],
                    rules: self.rules,
                    location: self.location,
                };
            }
            rect
        } else {
            // Dummy out unused data
            Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            }
        };
        let offset = num_children(&self.root) as isize - 1;
        QueryIterator {
            nodes: vec![(self.root, offset, rect)],
            rules: self.rules,
            location: self.location,
        }
    }

    pub fn next(self) -> Option<Node<E>> {
        self.matches().next()
    }
}

pub struct QueryIterator<'a, E: Extension + 'a> {
    nodes: Vec<(Node<E>, isize, Rect)>,
    rules: Vec<Rule<'a, E>>,
    location: Option<AtLocation>,
}

fn num_children<E: Extension>(node: &Node<E>) -> usize {
    let inner = node.inner.borrow();
    if let NodeValue::Element(ref e) = inner.value {
        e.children.len()
    } else {
        0
    }
}

impl<'a, E> Iterator for QueryIterator<'a, E>
    where E: Extension
{
    type Item = Node<E>;
    fn next(&mut self) -> Option<Node<E>> {
        enum Action<E: Extension> {
            Nothing,
            Pop,
            Push(Node<E>, Rect),
            Remove(Node<E>),
        }

        'search: loop {
            let action = if let Some(cur) = self.nodes.last_mut() {
                if cur.1 == -1 {
                    Action::Remove(cur.0.clone())
                } else {
                    let inner = cur.0.inner.borrow();
                    if let NodeValue::Element(ref e) = inner.value {
                        cur.1 -= 1;
                        if let Some(node) = e.children.get((cur.1 + 1) as usize) {
                            if let Some(loc) = self.location {
                                let mut rect = cur.2;
                                let p_rect = cur.2;
                                let p = node.parent()?.inner;
                                let inner = p.borrow();
                                let self_inner = node.inner.borrow();

                                rect.x += self_inner.draw_rect.x;
                                rect.y += self_inner.draw_rect.y;
                                rect.width = self_inner.draw_rect.width;
                                rect.height = self_inner.draw_rect.height;

                                rect.x += inner.scroll_position.0 as i32;
                                rect.y += inner.scroll_position.1 as i32;
                                if inner.clip_overflow {
                                    if rect.x < p_rect.x {
                                        rect.width -= p_rect.x - rect.x;
                                        rect.x = p_rect.x;
                                    }
                                    if rect.y < p_rect.y {
                                        rect.height -= p_rect.y - rect.y;
                                        rect.y = p_rect.y;
                                    }
                                    if rect.x + rect.width >= p_rect.x + p_rect.width {
                                        rect.width = (p_rect.x + p_rect.width) - rect.x;
                                    }
                                    if rect.y + rect.height >= p_rect.y + p_rect.height {
                                        rect.height = (p_rect.y + p_rect.height) - rect.y;
                                    }
                                }
                                if loc.x < rect.x || loc.x >= rect.x + rect.width || loc.y < rect.y
                                    || loc.y >= rect.y + rect.height
                                {
                                    Action::Nothing
                                } else {
                                    Action::Push(node.clone(), rect)
                                }
                            } else {
                                Action::Push(
                                    node.clone(),
                                    Rect {
                                        x: 0,
                                        y: 0,
                                        width: 0,
                                        height: 0,
                                    },
                                )
                            }
                        } else {
                            unreachable!()
                        }
                    } else {
                        Action::Pop
                    }
                }
            } else {
                return None;
            };

            let node = match action {
                Action::Nothing => continue 'search,
                Action::Pop => {
                    self.nodes.pop();
                    continue 'search;
                }
                Action::Push(node, rect) => {
                    self.nodes
                        .push((node.clone(), num_children(&node) as isize - 1, rect));
                    continue 'search;
                }
                Action::Remove(node) => {
                    self.nodes.pop();
                    node
                }
            };

            let mut cur = node.clone();
            for rule in self.rules.iter().rev() {
                match rule {
                    Rule::Text => if let NodeValue::Element(_) = cur.inner.borrow().value {
                        continue 'search;
                    },
                    Rule::Name(n) => if let NodeValue::Element(ref e) = cur.inner.borrow().value {
                        if e.name != *n {
                            continue 'search;
                        }
                    } else {
                        continue 'search;
                    },
                    Rule::Property(ref k, ref val) => {
                        let inner = cur.inner.borrow();
                        let ok = match (inner.properties.get(&**k), val) {
                            (Some(Value::Integer(a)), ValueRef::Integer(b)) => a == b,
                            (Some(Value::Float(a)), ValueRef::Float(b)) => a == b,
                            (Some(Value::Boolean(a)), ValueRef::Boolean(b)) => a == b,
                            (Some(Value::String(a)), ValueRef::String(b)) => a == b,
                            (Some(Value::ExtValue(a)), ValueRef::ExtValue(b)) => a == *b,
                            _ => false,
                        };
                        if !ok {
                            continue 'search;
                        }
                    }
                    Rule::Child => {
                        // Reversed so go up a level instead
                        let parent = cur.inner.borrow().parent.as_ref().and_then(|v| v.upgrade());
                        if let Some(parent) = parent {
                            cur = Node { inner: parent };
                        }
                    }
                }
            }
            return Some(node);
        }
    }
}

#[test]
fn test() {
    let doc = syntax::desc::Document::parse(
        r#"
panel {
    icon(type="warning")
    icon(type="warning")
    icon(type="cake")
    icon(type="warning")
    icon(type="test")
}

"#,
    ).unwrap();
    let node = Node::<tests::TestExt>::from_document(doc);

    for n in node.query()
        .name("panel")
        .child()
        .name("icon")
        .property("type", "warning")
        .matches()
    {
        assert_eq!(n.name(), Some("icon".to_owned()));
        assert_eq!(&*n.get_property_ref::<String>("type").unwrap(), "warning");
    }
}
