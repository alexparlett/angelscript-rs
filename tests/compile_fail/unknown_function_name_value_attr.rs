//! Test unknown function name-value attribute error.

use angelscript::function;

#[function(unknown_key = "value")]
fn test_func() {}

fn main() {}
