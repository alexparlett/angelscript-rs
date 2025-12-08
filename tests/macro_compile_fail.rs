//! Compile-fail tests for macro error paths.
//!
//! These tests verify that the macros produce helpful error messages
//! when used incorrectly.

#[test]
fn macro_compile_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
