#[allow(warnings)]
mod bindings;

use bindings::exports::component::ipfs::client::{Data, Cid, Guest};

struct Component;

impl Guest for Component {
    fn put(data: Data) -> Cid {
        return "doop doop doop, putting same data".into()
    }
    fn get(cid: Cid) -> Option<Data> {
        return Some("doop doop doop, getting same data".into())
    }
}

bindings::export!(Component with_types_in bindings);
