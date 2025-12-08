//! Test funcdef with non-function type error.

use angelscript::funcdef;

#[funcdef]
type TestCallback = i32;

fn main() {}
