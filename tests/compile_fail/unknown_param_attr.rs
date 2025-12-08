//! Test unknown param attribute error.

use angelscript::function;

#[function(generic)]
#[param(unknown_param_attr)]
fn test_func(_value: i32) {}

fn main() {}
