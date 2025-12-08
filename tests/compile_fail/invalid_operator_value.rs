//! Test non-path operator value error.

use angelscript::{Any, function};

#[derive(Any)]
struct Test;

impl Test {
    #[function(operator = "Add")]
    fn op_add(&self) -> i32 { 0 }
}

fn main() {}
