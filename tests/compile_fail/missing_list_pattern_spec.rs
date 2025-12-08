//! Test missing list_pattern specification error.

use angelscript::{Any, function};

#[derive(Any)]
struct Test;

impl Test {
    #[function(list_construct)]
    #[list_pattern()]
    fn from_list(&mut self, _size: i32) {}
}

fn main() {}
