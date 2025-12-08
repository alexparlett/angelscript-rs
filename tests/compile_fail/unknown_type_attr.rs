//! Test unknown angelscript type attribute error.

use angelscript::Any;

#[derive(Any)]
#[angelscript(unknown_attr)]
struct Test {
    value: i32,
}

fn main() {}
