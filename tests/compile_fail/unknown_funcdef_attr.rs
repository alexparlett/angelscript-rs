//! Test unknown funcdef attribute error.

use angelscript::funcdef;

#[funcdef(unknown_attr = "value")]
type TestCallback = fn(i32) -> bool;

fn main() {}
