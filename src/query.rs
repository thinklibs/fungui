
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
        let rect = if self.location.is_some() {
            self.root.render_position()
                .unwrap_or(Rect{x: 0, y: 0, width: 0, height: 0})
        } else {
            // Dummy out unused data
            Rect{x: 0, y: 0, width: 0, height: 0}
        };
        QueryIterator {
            nodes: vec![(self.root, -1, rect)],
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

impl <RInfo> Iterator for QueryIterator<RInfo> {
    type Item = Node<RInfo>;
    fn next(&mut self) -> Option<Node<RInfo>> {

        enum Action<RInfo> {
            Pop,
            Push(Node<RInfo>, Rect),
            Nothing(Node<RInfo>, Rect),
        }

        'search:
        loop {
            let action = if let Some(cur) = self.nodes.last_mut() {
                if cur.1 == -1 {
                    cur.1 += 1;
                    Action::Nothing(cur.0.clone(), cur.2)
                } else {
                    let inner = cur.0.inner.borrow();
                    if let NodeValue::Element(ref e) = inner.value {
                        cur.1 += 1;
                        if let Some(node) = e.children.get((cur.1 - 1) as usize) {
                            let rect = if self.location.is_some() {
                                let mut rect = cur.2;
                                let p = node.parent().inner;
                                let inner = p.borrow();
                                let p_obj = match inner.render_object
                                    .as_ref()
                                {
                                    Some(v) => v,
                                    None => return None,
                                };
                                let self_inner = node.inner.borrow();
                                let s_obj = match self_inner.render_object
                                    .as_ref()
                                {
                                    Some(v) => v,
                                    None => return None,
                                };

                                rect.x += s_obj.draw_rect.x;
                                rect.y += s_obj.draw_rect.y;
                                rect.width = s_obj.draw_rect.width;
                                rect.height = s_obj.draw_rect.height;

                                rect.x += p_obj.scroll_position.0 as i32;
                                rect.y += p_obj.scroll_position.1 as i32;
                                if p_obj.clip_overflow {
                                    if rect.x < 0 {
                                        rect.width += rect.x;
                                        rect.x = 0;
                                    }
                                    if rect.y < 0 {
                                        rect.height += rect.y;
                                        rect.y = 0;
                                    }
                                    if rect.x + rect.width >= p_obj.draw_rect.width {
                                        rect.width -=  (rect.x + rect.width) - p_obj.draw_rect.width;
                                    }
                                    if rect.y + rect.height >= p_obj.draw_rect.height {
                                        rect.height -= (rect.y + rect.height) - p_obj.draw_rect.height;
                                    }
                                }
                                rect
                            } else {
                                Rect{x: 0, y: 0, width: 0, height: 0}
                            };
                            Action::Push(node.clone(), rect)
                        } else {
                            Action::Pop
                        }
                    } else {
                        Action::Pop
                    }
                }
            } else {
                return None;
            };

            let (node, rect) = match action {
                Action::Pop => {
                    self.nodes.pop();
                    continue 'search;
                },
                Action::Push(node, rect) => {
                    self.nodes.push((node.clone(), -1, rect));
                    (node, rect)
                },
                Action::Nothing(node, rect) => {
                    (node, rect)
                },
            };

            if let Some(loc) = self.location {
                if loc.x < rect.x || loc.x >= rect.x + rect.width
                    || loc.y < rect.y || loc.y >= rect.y + rect.height
                {
                    continue;
                }
            }

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