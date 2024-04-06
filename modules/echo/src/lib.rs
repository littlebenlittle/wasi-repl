#[allow(warnings)]
mod bindings;

use bindings::Guest;

struct Component {}

impl Guest for Component {
    fn eval(expr: Vec<String>) -> String {
        expr.join("")
    }
}

bindings::export!(Component with_types_in bindings);