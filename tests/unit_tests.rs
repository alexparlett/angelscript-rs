//! Integration tests for AngelScript using Unit as the entry point.
//!
//! These tests validate the full build pipeline (parsing + compilation)
//! against complete AngelScript programs.

use angelscript::{Context, Unit};
use std::path::PathBuf;
use std::sync::Arc;

/// Load a test script from the test_scripts directory.
fn load_script(filename: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_scripts")
        .join(filename);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e))
}

/// Helper to build a module from a single test script.
fn build_script(filename: &str) -> Unit {
    let ctx = Context::with_default_modules().unwrap();
    let ctx = Arc::new(ctx);
    let mut unit = ctx.create_unit().unwrap();
    unit.add_source(filename, load_script(filename))
        .expect("Failed to add source");
    unit.build().expect("Failed to build module");
    unit
}

// =============================================================================
// Basic Programs
// =============================================================================

#[test]
fn test_hello_world() {
    let module = build_script("hello_world.as");
    assert!(module.is_built());
    // Should have 1 function: main()
    assert_eq!(module.function_count(), 1, "Expected 1 function (main)");
}

#[test]
fn test_literals() {
    let module = build_script("literals.as");
    assert!(module.is_built());
    // Should have 1 function: testLiterals()
    assert_eq!(
        module.function_count(),
        1,
        "Expected 1 function (testLiterals)"
    );
}

#[test]
fn test_operators() {
    let module = build_script("operators.as");
    assert!(module.is_built());
    // Should have 1 function: testOperators()
    assert_eq!(
        module.function_count(),
        1,
        "Expected 1 function (testOperators)"
    );
}

#[test]
fn test_control_flow() {
    let module = build_script("control_flow.as");
    assert!(module.is_built());
    // Contains multiple test functions with control flow statements
    assert!(module.function_count() >= 1, "Expected at least 1 function");
}

#[test]
fn test_functions() {
    let module = build_script("functions.as");
    assert!(module.is_built());
    // Should have multiple functions with various parameter patterns
    assert!(
        module.function_count() >= 10,
        "Expected at least 10 functions with various signatures"
    );
}

#[test]
fn test_types() {
    let module = build_script("types.as");
    assert!(module.is_built());
    // Should contain type alias declarations and auto type usage
}

// =============================================================================
// Object-Oriented Programming
// =============================================================================

#[test]
fn test_class_basic() {
    let module = build_script("class_basic.as");
    assert!(module.is_built());
    // Should have basic class declarations with methods
    assert!(module.function_count() >= 1, "Expected at least 1 function");
}

#[test]
fn test_inheritance() {
    let module = build_script("inheritance.as");
    assert!(module.is_built());
    // Should have classes with inheritance
    assert!(module.function_count() >= 1, "Expected at least 1 function");
}

#[test]
#[ignore = "requires class field access and default constructors"]
fn test_interface() {
    let module = build_script("interface.as");
    assert!(module.is_built());
    // Should have interface declarations
}

#[test]
fn test_properties() {
    let module = build_script("properties.as");
    assert!(module.is_built());
    // Should have classes with properties
}

#[test]
fn test_enum() {
    let module = build_script("enum.as");
    assert!(module.is_built());
    // Should have enum declarations
}

// =============================================================================
// Complex Structures
// =============================================================================

#[test]
fn test_nested() {
    let module = build_script("nested.as");
    assert!(module.is_built());
    // Should have nested namespace and class declarations
}

#[test]
fn test_using_namespace() {
    let module = build_script("using_namespace.as");
    assert!(module.is_built());
    // Should have multiple namespaces with functions
    assert!(
        module.function_count() >= 5,
        "Expected at least 5 functions across namespaces"
    );
}

#[test]
fn test_expressions() {
    let module = build_script("expressions.as");
    assert!(module.is_built());
    // Should have functions demonstrating various expression types
    assert!(module.function_count() >= 1, "Expected at least 1 function");
}

#[test]
fn test_templates() {
    let module = build_script("templates.as");
    assert!(module.is_built());
    // Should have template type usage
}

#[test]
fn test_lambdas() {
    let module = build_script("lambdas.as");
    assert!(module.is_built());
    // Should have functions using lambda expressions
    assert!(module.function_count() >= 1, "Expected at least 1 function");
}

// =============================================================================
// Real-World Programs
// =============================================================================

#[test]
fn test_game_logic() {
    let module = build_script("game_logic.as");
    assert!(module.is_built());
    // Should have game classes with logic functions
    assert!(
        module.function_count() >= 3,
        "Expected at least 3 functions"
    );
}

#[test]
fn test_utilities() {
    let module = build_script("utilities.as");
    assert!(module.is_built());
    // Should have utility functions
    assert!(
        module.function_count() >= 10,
        "Expected at least 10 utility functions"
    );
}

#[test]
fn test_data_structures() {
    let module = build_script("data_structures.as");
    assert!(module.is_built());
    // Should have data structure implementations
    assert!(module.function_count() >= 1, "Expected at least 1 function");
}

// =============================================================================
// Performance / Stress Tests
// =============================================================================

#[test]
fn test_large_function() {
    let module = build_script("large_function.as");
    assert!(module.is_built());
    // Single large function with many statements
    assert_eq!(module.function_count(), 1, "Expected 1 large function");
}

#[test]
fn test_many_functions() {
    let module = build_script("many_functions.as");
    assert!(module.is_built());
    // Should have 60 functions total (various types and helpers)
    assert_eq!(module.function_count(), 60, "Expected exactly 60 functions");
}

#[test]
fn test_performance_large_500() {
    let module = build_script("performance/large_500.as");
    assert!(module.is_built());
    // Complex performance test with many classes and functions
    assert!(
        module.function_count() >= 10,
        "Expected at least 10 functions"
    );
}

#[test]
#[ignore = "broken forward refs"]
fn test_performance_xlarge_1000() {
    let module = build_script("performance/xlarge_1000.as");
    assert!(module.is_built());
    // Large performance test with complex class hierarchies
    assert!(
        module.function_count() >= 20,
        "Expected at least 20 functions"
    );
}

#[test]
#[ignore = "broken forward refs"]
fn test_performance_xxlarge_5000() {
    let module = build_script("performance/xxlarge_5000.as");
    assert!(module.is_built());
    // Very large performance test
    assert!(
        module.function_count() >= 50,
        "Expected at least 50 functions"
    );
}
