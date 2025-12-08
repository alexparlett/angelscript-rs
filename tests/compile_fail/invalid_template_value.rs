//! Test non-string template value error.

use angelscript::function;

#[function(template = 123)]
fn test_func() {}

fn main() {}
