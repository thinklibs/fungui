
/// Used to create nodes inline without parsing a document
/// at runtime.
///
/// The syntax matches the node description format apart from
/// a few differences.
///
/// * Text can't be inline, it must be wrapped with `@text("hello")`
/// * String attributes currently need to be `.to_owned()` as
///   it expects `String` not `&str`.
///
/// # Examples
///
/// ```rust
/// # #[macro_use] extern crate stylish;
/// # fn main() {
/// # let _ : stylish::Node<()> =
/// node!{
///     panel(x=5, y=16, width=300, height=50) {
///         icon
///         title {
///             @text("Testing")
///         }
///     }
/// };
/// # }
/// ```
///
/// ```rust
/// # #[macro_use] extern crate stylish;
/// # fn main() {
/// # let _ : stylish::Node<()> =
/// node!{
///     @text("Hello world")
/// };
/// # }
/// ```
#[macro_export]
macro_rules! node {
    // Helper rules
    (@parent($parent:expr)) => {};

    (@parent($parent:expr) @text($txt:expr)) => ({
        $parent.add_child($crate::Node::new_text($txt));
    });
    (@parent($parent:expr) @text($txt:expr) $($other:tt)*) => ({
        $parent.add_child($crate::Node::new_text($txt));
        node!(@parent($parent) $($other)*);
    });

    (@parent($parent:expr) $name:ident (
        $($key:ident = $val:expr),*
    ) {
        $($inner:tt)*
    }) => ({
        let node = node!($name($($key = $val),*) {
            $($inner)*
        });
        $parent.add_child(node);
    });
    (@parent($parent:expr) $name:ident (
        $($key:ident = $val:expr),*
    ) {
        $($inner:tt)*
    } $($other:tt)*) => ({
        let node = node!($name($($key = $val),*) {
            $($inner)*
        });
        $parent.add_child(node);
        node!(@parent($parent) $($other)*);
    });

    (@parent($parent:expr) $name:ident (
        $($key:ident = $val:expr),*
    )) => ({
        let node = node!($name($($key = $val),*));
        $parent.add_child(node);
    });
    (@parent($parent:expr) $name:ident (
        $($key:ident = $val:expr),*
    ) $($other:tt)*) => ({
        let node = node!($name($($key = $val),*));
        $parent.add_child(node);
        node!(@parent($parent) $($other)*);
    });

    (@parent($parent:expr) $name:ident {
        $($inner:tt)*
    }) => ({
        let node = node!($name {
            $($inner)*
        });
        $parent.add_child(node);
    });
    (@parent($parent:expr) $name:ident {
        $($inner:tt)*
    } $($other:tt)*) => ({
        let node = node!($name {
            $($inner)*
        });
        $parent.add_child(node);
        node!(@parent($parent) $($other)*);
    });

    (@parent($parent:expr) $name:ident) => ({
        let node = node!($name);
        $parent.add_child(node);
    });
    (@parent($parent:expr) $name:ident $($other:tt)*) => ({
        let node = node!($name);
        $parent.add_child(node);
        node!(@parent($parent) $($other)*);
    });

    // Actual rules
    (@text($txt:expr)) => (
        $crate::Node::new_text($txt)
    );

    ($name:ident (
        $($key:ident = $val:expr),*
    ) {
        $($inner:tt)*
    }) => ({
        let node = $crate::Node::new(stringify!($name));
        $(
            node.set_property(stringify!($key), $val);
        )*
        node!(@parent(node) $($inner)*);
        node
    });
    ($name:ident (
        $($key:ident = $val:expr),*
    )) => ({
        let node = $crate::Node::new(stringify!($name));
        $(
            node.set_property(stringify!($key), $val);
        )*
        node
    });
    ($name:ident {
        $($inner:tt)*
    }) => ({
        let node = $crate::Node::new(stringify!($name));
        node!(@parent(node) $($inner)*);
        node
    });
    ($name:ident) => ({
        $crate::Node::new(stringify!($name))
    });
}

/// Allows for the creation of queries in a similar format
/// as style rules.
///
/// The syntax matches the node style format apart from
/// a few differences.
///
/// * Text can't be inline, it must be wrapped with `@text("hello")`
/// * String attributes currently need to be `.to_owned()` as
///   it expects `String` not `&str`.
///
/// # Examples
///
/// ```rust
/// # #[macro_use] extern crate stylish;
/// # use stylish::Node;
/// # fn main() {
/// # let node : Node<()> =
/// # node!{
/// #     panel(x=5, y=16, width=300, height=50) {
/// #         icon
/// #         title {
/// #             @text("Testing")
/// #         }
/// #     }
/// # };
/// assert_eq!(
///     query!(node, panel(width=300) > title > @text)
///         .next().and_then(|v| v.text()),
///     Some("Testing".to_owned())
/// );
/// # }
/// ```
#[macro_export]
macro_rules! query {

    (@target($query:expr), ) => (
        $query
    );

    (@target($query:expr), @text (
        $($key:ident = $val:expr),*
    ) > $($other:tt)*) => (
        query!(@target($query.text()
        $(
            .property(stringify!($key), $val)
        )*.child()), $($other)*)
    );
    (@target($query:expr), @text > $($other:tt)*) => (
        query!(@target($query.text().child()), $($other)*)
    );
    (@target($query:expr), @text (
        $($key:ident = $val:expr),*
    )) => (
        $query.text()
        $(
            .property(stringify!($key), $val)
        )*
    );
    (@target($query:expr), @text) => (
        $query.text()
    );

    (@target($query:expr), $name:ident (
        $($key:ident = $val:expr),*
    ) > $($other:tt)*) => (
        query!(@target($query.name(stringify!($name))
        $(
            .property(stringify!($key), $val)
        )*.child()), $($other)*)
    );
    (@target($query:expr), $name:ident > $($other:tt)*) => (
        query!(@target($query.name(stringify!($name)).child()), $($other)*)
    );
    (@target($query:expr), $name:ident (
        $($key:ident = $val:expr),*
    )) => (
        $query.name(stringify!($name))
        $(
            .property(stringify!($key), $val)
        )*
    );
    (@target($query:expr), $name:ident) => (
        $query.name(stringify!($name))
    );

    ($node:expr, $($other:tt)*) => ({
        let query = $node.query();
        query!(@target(query), $($other)*)
    });
}

#[test]
fn test_query_macro() {
    let node: super::Node<()> = node!{
        test {
            inner {
                @text("wrong".to_owned())
            }
            inner(a=5) {
                @text("hello".to_owned())
            }
        }
    };
    assert_eq!(
        query!(node, test > inner(a=5) > @text)
            .next()
            .and_then(|v| v.text()),
        Some("hello".to_owned())
    )
}
