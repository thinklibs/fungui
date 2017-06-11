
use syntax::Position;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }
    links {
    }

    foreign_links {
    }
    errors {
        UnknownVariable(name: String, position: Position) {}
        CantOp(op: String,  position: Position) {}
        UnknownFunction(name: String, position: Position) {
            display("Unknown function {} at {}", name, position)
        }
        MissingParameter(name: &'static str) {}
        IncorrectType(name: &'static str, wanted: &'static str) {}
        FunctionFailed(position: Position) {}
    }
}