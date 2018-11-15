use super::*;

use std::hash::{Hash, Hasher};

pub(crate) type SFunc<E> = Box<for<'a> Fn(&mut (Iterator<Item=Result<Value<E>, Error<'a>>> + 'a)) -> Result<Value<E>, Error<'a>> + 'static>;

/// Stores rules, functions and layouts needed for computing styles
pub struct Styles<E: Extension> {
    pub(crate) _ext: ::std::marker::PhantomData<E>,
    pub(crate) static_keys: FnvHashMap<&'static str, StaticKey>,
    pub(crate) rules: Rules<E>,
    pub(crate) funcs: FnvHashMap<StaticKey, SFunc<E>>,
    pub(crate) layouts: FnvHashMap<&'static str, Box<Fn() -> Box<BoxLayoutEngine<E>>>>,
    pub(crate) next_rule_id: u32,
    // Stored here for reuse to save on allocations
    pub(crate) used_keys: FnvHashSet<StaticKey>,
}

impl <E: Extension> Styles<E> {
    #[inline]
    #[doc(hidden)]
    pub fn key_was_used(&self, key: &StaticKey) -> bool {
        self.used_keys.contains(key)
    }

    pub(crate) fn load_styles<'a>(&mut self, name: &str, doc: syntax::style::Document<'a>) -> Result<(), syntax::PError<'a>>{
        for rule in doc.rules {
            let id = self.next_rule_id;
            self.next_rule_id = self.next_rule_id.wrapping_add(1);
            self.rules.add(id, &mut self.static_keys, name, rule)?;
        }
        Ok(())
    }
}

#[derive(Clone, Eq, Debug)]
pub struct RuleKey {
    pub inner: RuleKeyBorrow<'static>,
}

impl Hash for RuleKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl <'a> std::borrow::Borrow<RuleKeyBorrow<'a>> for RuleKey {
    fn borrow(&self) -> &RuleKeyBorrow<'a> {
        &self.inner
    }
}

impl PartialEq for RuleKey {
    fn eq(&self, other: &RuleKey) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl <'a> PartialEq<RuleKeyBorrow<'a>> for RuleKey {
    fn eq(&self, other: &RuleKeyBorrow<'a>) -> bool {
        self.inner.eq(other)
    }
}

#[derive(Clone, Eq, Debug)]
pub enum RuleKeyBorrow<'a> {
    Element(String),
    ElementBorrow(&'a str),
    Text,
}
impl <'a> PartialEq for RuleKeyBorrow<'a> {
    fn eq(&self, other: &RuleKeyBorrow<'a>) -> bool {
        match (self, other) {
            (RuleKeyBorrow::Element(ref a), RuleKeyBorrow::Element(ref b)) => a == b,
            (RuleKeyBorrow::ElementBorrow(ref a), RuleKeyBorrow::Element(ref b)) => a == b,
            (RuleKeyBorrow::Element(ref a), RuleKeyBorrow::ElementBorrow(ref b)) => a == b,
            (RuleKeyBorrow::ElementBorrow(ref a), RuleKeyBorrow::ElementBorrow(ref b)) => a == b,
            (RuleKeyBorrow::Text, RuleKeyBorrow::Text) => true,
            _ => false,
        }
    }
}

impl <'a> PartialEq<RuleKey> for RuleKeyBorrow<'a> {
    fn eq(&self, other: &RuleKey) -> bool {
        self.eq(&other.inner)
    }
}

impl <'a> Hash for RuleKeyBorrow<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            RuleKeyBorrow::Element(ref e) => {
                state.write_u8(0);
                e.as_str().hash(state);
            },
            RuleKeyBorrow::ElementBorrow(ref e) => {
                state.write_u8(0);
                e.hash(state);
            },
            RuleKeyBorrow::Text => {
                state.write_u8(1);
            },
        }
    }
}

/// Used for quick lookups into possible matches
/// for an element.
///
/// This wont check properties as its only ment to
/// reduce the search space.
pub struct Rules<E: Extension> {
    next: FnvHashMap<RuleKey, Rules<E>>,
    // Set of possible matches
    matches: Vec<Rc<Rule<E>>>,
}

#[derive(Debug)]
pub enum ValueMatcher {
    Boolean(bool),
    Integer(i32),
    Float(f64),
    String(String),
    Exists,
}

impl <E> Rules<E>
    where E: Extension
{
    pub fn new() -> Rules<E> {
        Rules {
            next: FnvHashMap::default(),
            matches: Vec::new(),
        }
    }

    fn add<'a>(&mut self, id: u32, keys: &mut FnvHashMap<&'static str, StaticKey>, name: &str, rule: syntax::style::Rule<'a>) -> Result<(), syntax::PError<'a>> {
        // Work in reverse to make lookups faster
        let mut current = self;
        for m in rule.matchers.iter().rev() {
            let key = match m.0 {
                syntax::style::Matcher::Text => RuleKeyBorrow::Text,
                syntax::style::Matcher::Element(ref e) => RuleKeyBorrow::Element(e.name.name.into()),
            };
            let tmp = current;
            let next = tmp.next.entry(RuleKey{inner: key}).or_insert_with(Rules::new);
            current = next;
        }
        let mut property_replacer = FnvHashMap::default();
        let mut matchers = Vec::with_capacity(rule.matchers.len());
        for (depth, m) in rule.matchers.into_iter().rev().enumerate() {
            let key = match m.0 {
                syntax::style::Matcher::Text => RuleKeyBorrow::Text,
                syntax::style::Matcher::Element(ref e) => RuleKeyBorrow::Element(e.name.name.into()),
            };
            let mut properties = Vec::with_capacity(m.1.len());
            for (k, v) in m.1 {
                use syntax::style::Value as SVal;
                let val = match v.value {
                    SVal::Boolean(b) => ValueMatcher::Boolean(b),
                    SVal::Integer(i) => ValueMatcher::Integer(i),
                    SVal::Float(f) => ValueMatcher::Float(f),
                    SVal::String(s) => ValueMatcher::String(unescape(s)),
                    SVal::Variable(n) => {
                        property_replacer.insert(n.name.to_owned(), (depth, k.name.to_owned()));
                        ValueMatcher::Exists
                    }
                };
                properties.push((k.name.to_owned(), val));
            }
            matchers.push((RuleKey{inner: key}, properties));
        }

        let mut styles = FnvHashMap::with_capacity_and_hasher(rule.styles.len(), Default::default());
        let mut uses_parent_size = false;
        for (k, e) in rule.styles {
            let key = match keys.get(k.name) {
                Some(val) => val,
                None => return Err(syntax::Errors::new(
                    k.position.into(),
                    syntax::Error::Message(syntax::Info::Borrowed("Unknown style key")),
                )),
            };
            styles.insert(*key, Expr::from_style(keys, &property_replacer, &mut uses_parent_size, e)?);
        }
        current.matches.push(Rc::new(Rule {
            id,
            name: name.into(),
            matchers,
            styles,
            uses_parent_size,
        }));
        Ok(())
    }

    // Kinda expensive but shouldn't be common
    pub fn remove_all_by_name(&mut self, name: &str) {
        self.next.retain(|_k, v| {
            v.remove_all_by_name(name);
            !v.matches.is_empty()
        });
        self.matches.retain(|v| v.name != name);
    }

    pub(super) fn get_possible_matches(&self, node: &NodeChain<E>, out: &mut Vec<Rc<Rule<E>>>) {
        let mut current = self;
        let mut node = Some(node);
        while let Some(n) = node.take() {
            {
                let key = match n.value {
                    NCValue::Text(_) => RuleKeyBorrow::Text,
                    NCValue::Element(ref e) => RuleKeyBorrow::ElementBorrow(e),
                };
                current = if let Some(v) = current.next.get(&key) {
                    v
                } else {
                    break
                };
                out.extend(current.matches.iter().cloned());
            }
            node = n.parent;
        }
        out.sort_unstable_by_key(|v| v.id);
    }
}

/// A rule which contains a set of matchers to compare against
/// the properties of a node and parents and a set of styles to
/// apply if matched.
pub struct Rule<E: Extension> {
    id: u32,
    name: String,
    pub(crate) matchers: Vec<(RuleKey, Vec<(String, ValueMatcher)>)>,
    #[doc(hidden)]
    // Used by the `eval!` macro
    pub styles: FnvHashMap<StaticKey, Expr<E>>,
    pub(crate) uses_parent_size: bool,
}

impl <E> Rule<E>
    where E: Extension
{
    pub(super) fn test(&self, node: &NodeChain<E>) -> bool {
        let mut node = Some(node);
        for (_rkey, props) in &self.matchers {
            if let Some(n) = node.take() {
                // Key doesn't need checking because `get_possible_matches` will filter
                // that

                for (key, vm) in props {
                    if let Some(val) = n.properties.get(key) {
                        let same = match (vm, val) {
                            (ValueMatcher::Boolean(a), Value::Boolean(b)) => *a == *b,
                            (ValueMatcher::Integer(a), Value::Integer(b)) => *a == *b,
                            (ValueMatcher::Integer(a), Value::Float(b)) => *a as f64 == *b,
                            (ValueMatcher::Float(a), Value::Float(b)) => *a == *b,
                            (ValueMatcher::Float(a), Value::Integer(b)) => *a == *b as f64,
                            (ValueMatcher::String(ref a), Value::String(ref b)) => a == b,
                            (ValueMatcher::Exists, _) => true,
                            (_, _) => false,
                        };
                        if !same {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                node = n.parent;
            } else {
                return false;
            }
        }
        true
    }
}