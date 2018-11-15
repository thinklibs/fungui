

/// The error type used in FunGUI
#[derive(Debug)]
pub enum Error<'a> {
    /// An unknown variable was used
    UnknownVariable {
        /// The name of the variable
        name: &'a str,
    },
    /// An incompatible type was used with the given
    /// operator
    IncompatibleTypeOp {
        /// The operator
        op: &'static str,
        /// The incorrect type
        ty: &'static str,
    },
    /// An incompatible pair of types was used with the given
    /// operator
    IncompatibleTypesOp {
        /// The operator
        op: &'static str,
        /// The type of the left hand side
        left_ty: &'static str,
        /// The type of the right hand side
        right_ty: &'static str,
    },
    /// A custom reason
    Custom {
        /// The reason
        reason: String,
    },
    /// A custom reason without allocating
    CustomStatic {
        /// The reason
        reason: &'static str,
    },
    /// The parameter at the given position
    /// is missing
    MissingParameter {
        /// The parameter position
        position: i32,
        /// The parameter name
        name: &'static str,
    }
}