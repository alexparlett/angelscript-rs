//! Test unknown angelscript field attribute error.

use angelscript::Any;

#[derive(Any)]
struct Test {
    #[angelscript(unknown_field_attr)]
    value: i32,
}

fn main() {}
