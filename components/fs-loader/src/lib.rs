#[allow(warnings)]
mod bindings;

use bindings::Guest;

struct Component {}

impl Guest for Component {
    fn load(path: String) -> Result<Vec<u8>, String> {
        std::fs::read(path).or_else(|e| Result::Err(e.to_string()))
    }
    fn ls() -> Option<String> {
        let dir = match std::env::current_dir() {
            Ok(x) => x,
            Err(e) => return Some(e.to_string()),
        };
        let rd = match std::fs::read_dir(dir) {
            Ok(x) => x,
            Err(e) => return Some(e.to_string()),
        };
        for entry in rd {
            match entry {
                Ok(e) => println!("{}", e.file_name().into_string().unwrap()),
                Err(e) => println!("{:?}", e),
            }
        }
        None
    }
}

bindings::export!(Component with_types_in bindings);
