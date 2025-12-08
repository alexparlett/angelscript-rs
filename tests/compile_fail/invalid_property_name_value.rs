//! Test non-string property_name value error.

use angelscript::{Any, function};

#[derive(Any)]
struct Test;

impl Test {
    #[function(property, property_name = 123)]
    fn get_value(&self) -> i32 { 0 }
}

fn main() {}
