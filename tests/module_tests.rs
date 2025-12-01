//! Integration tests for AngelScript using ScriptModule as the entry point.
//!
//! These tests validate the full build pipeline (parsing + compilation)
//! against complete AngelScript programs.

use angelscript::ScriptModule;
use std::path::PathBuf;

/// Load a test script from the test_scripts directory.
fn load_script(filename: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_scripts")
        .join(filename);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e))
}

/// Helper to build a module from a single test script.
fn build_script(filename: &str) -> ScriptModule {
    let mut module = ScriptModule::new();
    module
        .add_source(filename, load_script(filename))
        .expect("Failed to add source");
    module.build().expect("Failed to build module");
    module
}

// =============================================================================
// Basic Programs
// =============================================================================

#[test]
fn test_hello_world() {
    let module = build_script("hello_world.as");
    assert!(module.is_built());
    assert!(module.function_count() >= 1);
}

#[test]
fn test_literals() {
    let module = build_script("literals.as");
    assert!(module.is_built());
    assert!(module.function_count() >= 1);
}

#[test]
fn test_operators() {
    let module = build_script("operators.as");
    assert!(module.is_built());
    assert!(module.function_count() >= 1);
}

#[test]
fn test_control_flow() {
    let module = build_script("control_flow.as");
    assert!(module.is_built());
    assert!(module.function_count() >= 1);
}

#[test]
fn test_functions() {
    let module = build_script("functions.as");
    assert!(module.is_built());
    assert!(module.function_count() >= 3);
}

#[test]
fn test_types() {
    let module = build_script("types.as");
    assert!(module.is_built());
}

// =============================================================================
// Object-Oriented Programming
// =============================================================================

#[test]
fn test_class_basic() {
    let module = build_script("class_basic.as");
    assert!(module.is_built());
    assert!(module.function_count() >= 1);
}

#[test]
fn test_inheritance() {
    let module = build_script("inheritance.as");
    assert!(module.is_built());
    assert!(module.function_count() >= 1);
}

#[test]
fn test_interface() {
    let module = build_script("interface.as");
    assert!(module.is_built());
}

#[test]
fn test_properties() {
    let module = build_script("properties.as");
    assert!(module.is_built());
}

#[test]
fn test_enum() {
    let module = build_script("enum.as");
    assert!(module.is_built());
}

// =============================================================================
// Complex Structures
// =============================================================================

#[test]
fn test_nested() {
    let module = build_script("nested.as");
    assert!(module.is_built());
}

#[test]
fn test_expressions() {
    let module = build_script("expressions.as");
    assert!(module.is_built());
    assert!(module.function_count() >= 1);
}

#[test]
fn test_templates() {
    let module = build_script("templates.as");
    assert!(module.is_built());
}

#[test]
fn test_lambdas() {
    let module = build_script("lambdas.as");
    assert!(module.is_built());
    assert!(module.function_count() >= 1);
}

// =============================================================================
// Real-World Programs
// =============================================================================

#[test]
fn test_game_logic() {
    let module = build_script("game_logic.as");
    assert!(module.is_built());
    assert!(module.function_count() >= 3);
}

#[test]
fn test_utilities() {
    let module = build_script("utilities.as");
    assert!(module.is_built());
    assert!(module.function_count() >= 10);
}

#[test]
fn test_data_structures() {
    let module = build_script("data_structures.as");
    assert!(module.is_built());
    assert!(module.function_count() >= 1);
}

// =============================================================================
// Performance / Stress Tests
// =============================================================================

#[test]
fn test_large_function() {
    let module = build_script("large_function.as");
    assert!(module.is_built());
    assert!(module.function_count() >= 1);
}

#[test]
fn test_many_functions() {
    let module = build_script("many_functions.as");
    assert!(module.is_built());
    assert!(module.function_count() >= 20);
}
