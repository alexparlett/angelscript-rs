//! Test unknown return attribute error.

use angelscript::{Any, function};

#[derive(Any)]
struct Test;

impl Test {
    #[function]
    #[returns(unknown_return_attr)]
    fn get_value(&self) -> i32 { 0 }
}

fn main() {}
