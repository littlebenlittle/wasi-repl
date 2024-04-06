#[allow(warnings)]
mod bindings;

use bindings::Guest;

struct Component {}

impl Guest for Component {
    fn load(path: String) -> Result<Vec<u8>, String> {
        std::fs::read(path).or_else(|e| Result::Err(e.to_string()))
    }
}

bindings::export!(Component with_types_in bindings);