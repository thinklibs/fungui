
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

    fn collect_nodes(out: &mut Vec<Node<RInfo>>, node: &Node<RInfo>) {
        let inner = node.inner.borrow();
        out.push(node.clone());
        if let NodeValue::Element(ref e) = inner.value {
            for c in e.children.iter() {
                Self::collect_nodes(out, c);
            }
        }
    }

    pub fn matches(self) -> QueryIterator<RInfo> {
        /// Collect nodes
        let mut nodes = Vec::new();
        Self::collect_nodes(&mut nodes, &self.root);
        QueryIterator {
            nodes: nodes,
            rules: self.rules,
            location: self.location,
        }
    }

    pub fn next(self) -> Option<Node<RInfo>> {
        self.matches().next()
    }
}

pub struct QueryIterator<RInfo> {
    nodes: Vec<Node<RInfo>>,
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
        'search:
        while let Some(node) = self.nodes.pop() {

            if let Some(loc) = self.location {
                let rect = match node.render_position() {
                    Some(v) => v,
                    None => continue,
                };

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
        None
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