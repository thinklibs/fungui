
use super::*;

pub struct Query<RInfo> {
    pub(crate) root: Node<RInfo>,
    pub(crate) rules: Vec<Rule>,
    pub(crate) location: Option<AtLocation>,
}

#[derive(Clone, Copy)]
pub(crate) struct AtLocation {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[derive(Debug, Clone)]
pub(crate) enum Rule {
    /// Matches against child nodes
    Child,
    /// Matches against the element's name
    Name(String),
    /// Matches against a property
    Property(String, Value),
    /// Matches against a text node
    Text,
}

impl <RInfo> Query<RInfo> {
    pub(super) fn new(node: Node<RInfo>) -> Query<RInfo> {
        Query {
            root: node,
            rules: vec![],
            location: None,
        }
    }

    pub fn name<S>(mut self, name: S) -> Query<RInfo>
        where S: Into<String>
    {
        self.rules.push(Rule::Name(name.into()));
        self
    }

    pub fn text(mut self) -> Query<RInfo>
    {
        self.rules.push(Rule::Text);
        self
    }

    pub fn property<S, V>(mut self, key: S, val: V) -> Query<RInfo>
        where V: PropertyValue,
              S: Into<String>
    {
        self.rules.push(Rule::Property(key.into(), val.convert_into()));
        self
    }

    pub fn child(mut self) -> Query<RInfo> {
        self.rules.push(Rule::Child);
        self
    }

    pub fn matches(self) -> QueryIterator<RInfo> {
        let rect = if let Some(loc) = self.location {
            let rect = self.root.render_position()
                .unwrap_or(Rect{x: 0, y: 0, width: 0, height: 0});

            if loc.x < rect.x || loc.x >= rect.x + rect.width
                || loc.y < rect.y || loc.y >= rect.y + rect.height
            {
                return QueryIterator {
                    nodes: vec![],
                    rules: self.rules,
                    location: self.location,
                }
            }
            rect
        } else {
            // Dummy out unused data
            Rect{x: 0, y: 0, width: 0, height: 0}
        };
        let offset = num_children(&self.root) as isize - 1;
        QueryIterator {
            nodes: vec![(self.root, offset, rect)],
            rules: self.rules,
            location: self.location,
        }
    }

    pub fn next(self) -> Option<Node<RInfo>> {
        self.matches().next()
    }
}

pub struct QueryIterator<RInfo> {
    nodes: Vec<(Node<RInfo>, isize, Rect)>,
    rules: Vec<Rule>,
    location: Option<AtLocation>,
}

impl <RInfo> Clone for QueryIterator<RInfo> {
    fn clone(&self) -> Self {
        QueryIterator {
            nodes: Clone::clone(&self.nodes),
            rules: Clone::clone(&self.rules),
            location: self.location,
        }
    }
}

fn num_children<T>(node: &Node<T>) -> usize {
    let inner = node.inner.borrow();
    if let NodeValue::Element(ref e) = inner.value {
        e.children.len()
    } else {
        0
    }
}

impl <RInfo> Iterator for QueryIterator<RInfo> {
    type Item = Node<RInfo>;
    fn next(&mut self) -> Option<Node<RInfo>> {

        enum Action<RInfo> {
            Nothing,
            Pop,
            Push(Node<RInfo>, Rect),
            Remove(Node<RInfo>),
        }

        'search:
        loop {
            let action = if let Some(cur) = self.nodes.last_mut() {
                if cur.1 == -1 {
                    Action::Remove(cur.0.clone())
                } else {
                    let inner = cur.0.inner.borrow();
                    if let NodeValue::Element(ref e) = inner.value {
                        cur.1 -= 1;
                        let len = e.children.len();
                        if let Some(node) = e.children.get(len - 1 - (cur.1 + 1) as usize) {
                            if let Some(loc) = self.location {
                                let mut rect = cur.2;
                                let p_rect = cur.2;
                                let p = node.parent().inner;
                                let inner = p.borrow();
                                let p_obj = match inner.render_object
                                    .as_ref()
                                {
                                    Some(v) => v,
                                    None => continue 'search,
                                };
                                let self_inner = node.inner.borrow();
                                let s_obj = match self_inner.render_object
                                    .as_ref()
                                {
                                    Some(v) => v,
                                    None => continue 'search,
                                };

                                rect.x += s_obj.draw_rect.x;
                                rect.y += s_obj.draw_rect.y;
                                rect.width = s_obj.draw_rect.width;
                                rect.height = s_obj.draw_rect.height;

                                rect.x += p_obj.scroll_position.0 as i32;
                                rect.y += p_obj.scroll_position.1 as i32;
                                if p_obj.clip_overflow {
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
                                if loc.x < rect.x || loc.x >= rect.x + rect.width
                                    || loc.y < rect.y || loc.y >= rect.y + rect.height
                                {
                                    Action::Nothing
                                } else {
                                    Action::Push(node.clone(), rect)
                                }
                            } else {
                                Action::Push(node.clone(), Rect{x: 0, y: 0, width: 0, height: 0})
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
                },
                Action::Push(node, rect) => {
                    self.nodes.push((node.clone(), num_children(&node) as isize - 1, rect));
                    continue 'search;
                },
                Action::Remove(node) => {
                    self.nodes.pop();
                    node
                },
            };

            let mut cur = node.clone();
            for rule in self.rules.iter().rev() {
                match *rule {
                    Rule::Text => {
                        if let NodeValue::Element(_) = cur.inner.borrow().value {
                            continue 'search;
                        }
                    }
                    Rule::Name(ref n) => {
                        if !cur.name().map_or(false, |v| *v == *n) {
                            continue 'search;
                        }
                    }
                    Rule::Property(ref k, ref val) => {
                        if !cur.get_property::<Value>(k).map_or(false, |v| v == *val) {
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
    let doc = syntax::desc::Document::parse(r#"
panel {
    icon(type="warning")
    icon(type="warning")
    icon(type="cake")
    icon(type="warning")
    icon(type="test")
}

"#).unwrap();
    let node = Node::<()>::from_document(doc);

    for n in node.query()
        .name("panel")
        .child()
        .name("icon")
        .property("type", "warning".to_owned())
        .matches()
    {
        assert_eq!(n.name(), Some("icon".to_owned()));
        assert_eq!(n.get_property::<String>("type"), Some("warning".to_owned()));
    }
}