// tests/integration_tests.rs
//! Integration tests for AngelScript parser
//!
//! These tests validate the parser against complete AngelScript programs,
//! ensuring all features work together correctly.

mod test_harness;

use test_harness::{TestHarness, AstCounter};

/// Test parsing basic AngelScript programs
#[test]
fn test_basic_program() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("hello_world.as");

    result.assert_success();

    let functions = result.get_functions();
    assert!(functions.len() >= 1, "Should have at least one function");
}

#[test]
fn test_literals_all_types() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("literals.as");

    result.assert_success();
    assert!(result.get_functions().len() >= 1);
}

#[test]
fn test_operators_precedence() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("operators.as");

    result.assert_success();

    // Count binary expressions to ensure operators are being parsed
    let counter = AstCounter::new().count_script(&result.script);
    assert!(
        counter.binary_expr_count >= 5,
        "Should have multiple binary expressions"
    );
}

#[test]
fn test_control_flow() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("control_flow.as");

    result.assert_success();

    let counter = AstCounter::new().count_script(&result.script);
    assert!(counter.if_count >= 1, "Should have if statements");
    assert!(counter.while_count >= 1, "Should have while loops");
    assert!(counter.for_count >= 1, "Should have for loops");
}

#[test]
fn test_functions_params() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("functions.as");


    result.assert_success();

    let functions = result.get_functions();
    assert!(functions.len() >= 3, "Should have multiple functions");

    // Check for functions with parameters
    let has_params = functions.iter().any(|f| !f.params.is_empty());
    assert!(has_params, "Should have functions with parameters");
}

#[test]
fn test_type_expressions() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("types.as");

    result.assert_success();

    // Should successfully parse complex type expressions
    let vars = result.get_global_vars();
    assert!(
        vars.len() >= 3,
        "Should have multiple variable declarations with different types"
    );
}

/// Test object-oriented programming features
#[test]
fn test_class_basic() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("class_basic.as");

    result.assert_success();

    let classes = result.get_classes();
    assert_eq!(classes.len(), 2, "Should have one class");
    assert!(!classes[0].members.is_empty(), "Class should have members");
}

#[test]
fn test_class_inheritance() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("inheritance.as");

    result.assert_success();

    let classes = result.get_classes();
    assert!(classes.len() >= 2, "Should have base and derived classes");

    // Check that at least one class has base classes
    let has_base = classes.iter().any(|c| !c.inheritance.is_empty());
    assert!(has_base, "Should have inheritance");
}

#[test]
fn test_interface() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("interface.as");

    result.assert_success();

    let interfaces = result.get_interfaces();
    assert!(interfaces.len() >= 1, "Should have at least one interface");
}

#[test]
fn test_properties() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("properties.as");

    result.assert_success();

    let classes = result.get_classes();
    assert!(classes.len() >= 1, "Should have a class with properties");
}

#[test]
fn test_enum_declaration() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("enum.as");

    result.assert_success();

    let enums = result.get_enums();
    assert!(enums.len() >= 1, "Should have at least one enum");
    assert!(
        !enums[0].enumerators.is_empty(),
        "Enum should have enumerators"
    );
}

/// Test complex nested structures
#[test]
fn test_nested_classes() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("nested.as");

    result.assert_success();

    let counter = AstCounter::new().count_script(&result.script);
    assert!(counter.class_count >= 2, "Should have nested classes >= 2 found {}", counter.class_count);
}

#[test]
fn test_complex_expressions() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("expressions.as");

    result.assert_success();

    let counter = AstCounter::new().count_script(&result.script);
    assert!(
        counter.binary_expr_count >= 10,
        "Should have complex expressions"
    );
}

#[test]
fn test_templates() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("templates.as");

    result.assert_success();

    // Templates should parse successfully including nested templates
    assert!(
        result.source_contains("array<"),
        "Should have template types"
    );
}

/// Test real-world-like programs
#[test]
fn test_game_logic() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("game_logic.as");

    result.assert_success();

    let counter = AstCounter::new().count_script(&result.script);
    assert!(counter.class_count >= 1, "Game logic should have classes");
    assert!(
        counter.function_count >= 3,
        "Game logic should have multiple functions"
    );
}

#[test]
fn test_utility_functions() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("utilities.as");

    result.assert_success();

    let counter = AstCounter::new().count_script(&result.script);
    assert!(
        counter.function_count >= 10,
        "Should have complex expressions"
    );
}

#[test]
fn test_data_structures() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("data_structures.as");

    result.assert_success();

    let classes = result.get_classes();
    assert!(classes.len() >= 2, "Should have data structure classes");
}


/// Performance tests (basic)
#[test]
fn test_large_function() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("large_function.as");

    result.assert_success();

    let counter = AstCounter::new().count_script(&result.script);
    // Large function should have many statements
    assert!(counter.if_count + counter.while_count + counter.for_count >= 10);
}

#[test]
fn test_many_functions() {
    let harness = TestHarness::new();
    let result = harness.load_and_parse("many_functions.as");

    result.assert_success();

    let functions = result.get_functions();
    assert!(functions.len() >= 20, "Should have many functions");
}