#![allow(missing_docs)]
use super::*;

pub enum TestExt{}

static CHAR: StaticKey = StaticKey("char");

impl Extension for TestExt {
    type NodeData = TestData;
    type Value = ();
    fn new_data() -> TestData {
        TestData {
            render_char: '#',
        }
    }

    fn style_properties<'a, F>(mut prop: F)
        where F: FnMut(StaticKey) + 'a
    {
        prop(CHAR);
    }

    fn update_data(styles: &Styles<TestExt>, nc: &NodeChain<TestExt>, rule: &Rule<TestExt>, data: &mut Self::NodeData) -> DirtyFlags {
        eval!(styles, nc, rule.CHAR => val => {
            if let Some(c) = val.convert::<String>() {
                data.render_char = c.chars().next().unwrap_or('~');
            } else {
                data.render_char = '~';
            }
        });

        DirtyFlags::empty()
    }

    fn reset_unset_data(used_keys: &FnvHashSet<StaticKey>, data: &mut Self::NodeData) -> DirtyFlags {
        if !used_keys.contains(&CHAR) {
            data.render_char = '~';
        }
        DirtyFlags::empty()
    }
}

pub struct TestData {
    render_char: char,
}

pub struct AsciiRender {
    width: usize,
    height: usize,
    data: Vec<char>,
    offsets: Vec<(i32, i32)>,
}

impl AsciiRender {
    pub fn new(width: usize, height: usize) -> AsciiRender {
        let data = vec!['#'; width * height];
        AsciiRender {
            width,
            height,
            data,
            offsets: vec![(0, 0)],
        }
    }

    pub fn as_string(&self) -> String {
        let mut out = String::with_capacity(self.width * self.height);
        for line in self.data.chunks(self.width) {
            out.extend(line);
            out.push('\n');
        }
        out.pop();
        out
    }
}

impl RenderVisitor<TestExt> for AsciiRender {

    fn visit(&mut self, node: &mut NodeInner<TestExt>) {
        let c = node.ext.render_char;
        let (lx, ly) = self.offsets.last().cloned().expect("Missing offset data");
        let ox = node.draw_rect.x + lx;
        let oy = node.draw_rect.y + ly;
        for y in 0 .. node.draw_rect.height {
            for x in 0 .. node.draw_rect.width {
                let idx = (ox + x) as usize + (oy + y) as usize * self.width;
                self.data[idx] = c;
            }
        }
        self.offsets.push((ox, oy));
    }
    fn visit_end(&mut self, _node: &mut NodeInner<TestExt>) {
        self.offsets.pop();
    }
}


#[test]
fn test() {
    let mut manager: Manager<TestExt> = Manager::new();
    manager.add_func_raw("add_two", |args| -> Result<_, _> {
        let val: i32 = args.next()
            .ok_or(Error::MissingParameter {
                position: 0,
                name: "value"
            })
            .and_then(|v| v)?
            .convert()
            .ok_or(Error::CustomStatic {
                reason: "Expected integer"
            })?;

        Ok(Value::Integer(val + 2))
    });
    let src = r#"
basic_abs {
    x = 2,
    y = 1,
    width = 4,
    height = 3,
    char = "@",
}
basic_abs(offset=ox) {
    x = add_two(ox),
}

inner {
    x = 1,
    y = 1,
    width = 1,
    height = 1,
    char = "+",
}
    "#;
    if let Err(err) = manager.load_styles("test", src) {
        let mut stdout = std::io::stdout();
        format_parse_error(stdout.lock(), src.lines(), err).unwrap();
        panic!("Styles failed to parse");
    }
    manager.add_node(node! {
        basic_abs
    });
    manager.add_node(node! {
        basic_abs(offset = 5) {
            inner
        }
    });

    manager.layout(20, 8);

    let mut render = AsciiRender::new(20, 8);
    manager.render(&mut render);

    let layout = render.as_string();
    println!("Layout: \n{}", layout);

    let expected_output = r##"
####################
##@@@@#@@@@#########
##@@@@#@+@@#########
##@@@@#@@@@#########
####################
####################
####################
####################
"##.trim();

    assert_eq!(layout, expected_output);
}