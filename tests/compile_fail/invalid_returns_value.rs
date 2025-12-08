//! Test non-path returns value error in function attribute.

use angelscript::function;

#[function(generic, returns = "i32")]
fn test_func() -> i32 { 0 }

fn main() {}
