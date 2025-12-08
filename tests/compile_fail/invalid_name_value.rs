//! Test non-string name value error.

use angelscript::function;

#[function(name = 123)]
fn test_func() {}

fn main() {}
