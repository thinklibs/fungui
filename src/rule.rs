use super::*;

pub struct RuleIter<'a, I, RInfo: 'a> {
    pub(crate) node: &'a Node<RInfo>,
    pub(crate) rules: I,
}

#[derive(Debug)]
pub struct Rule<'a> {
    pub(crate) syn: &'a syntax::style::Rule,
    pub(crate) vars: HashMap<String, Value>,
}

impl <'a> Rule<'a> {
    pub(crate) fn eval_value(&self, val: &syntax::style::Value) -> SResult<Value> {
        use syntax::style;
        Ok(match *val {
            style::Value::Float(f) => Value::Float(f),
            style::Value::Integer(i) => Value::Integer(i),
            style::Value::String(ref s) => Value::String(s.clone()),
            style::Value::Variable(ref name) => self.vars.get(&name.name)
                .cloned()
                .ok_or_else(|| ErrorKind::UnknownVariable(name.name.clone(), name.position))?,
        })
    }

    pub(crate) fn eval<T>(&self, styles: &Styles<T>, expr: &syntax::style::ExprType) -> SResult<Value> {
        use syntax::style;
        match expr.expr {
            style::Expr::Value(ref v) => self.eval_value(v),
            style::Expr::Add(ref l, ref r) => {
                let l = self.eval(styles, l)?;
                let r = self.eval(styles, r)?;
                match (l, r) {
                    (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l + r)),
                    (Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l + r)),
                    (Value::String(l), Value::String(r)) => Ok(Value::String(l + &r)),
                    _ => Err(ErrorKind::CantOp(
                        "add".into(),
                        expr.position,
                    ).into()),
                }
            },
            style::Expr::Sub(ref l, ref r) => {
                let l = self.eval(styles, l)?;
                let r = self.eval(styles, r)?;
                match (l, r) {
                    (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l - r)),
                    (Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l - r)),
                    _ => Err(ErrorKind::CantOp(
                        "subtract".into(),
                        expr.position,
                    ).into()),
                }
            },
            style::Expr::Mul(ref l, ref r) => {
                let l = self.eval(styles, l)?;
                let r = self.eval(styles, r)?;
                match (l, r) {
                    (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l * r)),
                    (Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l * r)),
                    _ => Err(ErrorKind::CantOp(
                        "multiply".into(),
                        expr.position,
                    ).into()),
                }
            },
            style::Expr::Div(ref l, ref r) => {
                let l = self.eval(styles, l)?;
                let r = self.eval(styles, r)?;
                match (l, r) {
                    (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l / r)),
                    (Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l / r)),
                    _ => Err(ErrorKind::CantOp(
                        "divide".into(),
                        expr.position,
                    ).into()),
                }
            },
            style::Expr::Neg(ref l) => {
                let l = self.eval(styles, l)?;
                match l {
                    Value::Float(l) => Ok(Value::Float(-l)),
                    Value::Integer(l) => Ok(Value::Integer(-l)),
                    _ => Err(ErrorKind::CantOp(
                        "negate".into(),
                        expr.position,
                    ).into()),
                }
            },
            style::Expr::Call(ref name, ref args) => {
                if let Some(func) = styles.funcs.get(&name.name) {
                    let args = args.iter()
                        .map(|v| self.eval(styles, &v))
                        .collect::<SResult<Vec<_>>>()?;
                    func(args)
                        .chain_err(|| ErrorKind::FunctionFailed(expr.position))
                } else {
                    Err(ErrorKind::UnknownFunction(name.name.clone(), name.position).into())
                }
            },
        }
    }

    pub(crate) fn get_value<T, V: PropertyValue>(&self, styles: &Styles<T>, name: &str) -> Option<V> {
        use syntax::Ident;
        let ident = Ident {
            name: name.into(),
            .. Default::default()
        };
        if let Some(expr) = self.syn.styles.get(&ident) {
            let val = self.eval(styles, expr);
            match val {
                Ok(val) => V::convert_from(&val),
                Err(err) => {
                    println!("{}", err);
                    None
                },
            }
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