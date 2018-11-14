

#[derive(Debug)]
pub enum Error<'a> {
    UnknownVariable {
        name: &'a str,
    },
    IncompatibleTypeOp {
        op: &'static str,
        ty: &'static str,
    },
    IncompatibleTypesOp {
        op: &'static str,
        left_ty: &'static str,
        right_ty: &'static str,
    },
    Custom {
        reason: String,
    },
    CustomStatic {
        reason: &'static str,
    },
    MissingParameter {
        position: i32,
        name: &'static str,
    }
}