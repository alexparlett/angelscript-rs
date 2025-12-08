//! Test unknown function attribute error.

use angelscript::function;

#[function(unknown_function_attr)]
fn test_func() {}

fn main() {}
