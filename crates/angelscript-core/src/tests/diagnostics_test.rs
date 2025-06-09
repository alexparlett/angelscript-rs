#[cfg(test)]
mod tests {
    use crate::core::diagnostics::{Diagnostic, DiagnosticKind, Diagnostics};
    use crate::core::engine::Engine;
    use crate::core::error::ScriptResult;
    use crate::types::enums::GetModuleFlags;

    #[test]
    fn test_diagnostics_with_successful_compilation() -> ScriptResult<()> {
        // Create the script engine
        let mut engine = Engine::create().expect("Failed to create script engine");
        let mut diagnostics = Diagnostics::new();

        // Set up diagnostic callback
        engine.set_diagnostic_callback(&mut diagnostics)?;

        // Create a module with valid script
        let module = engine.get_module("TestModule", GetModuleFlags::AlwaysCreate)?;

        let valid_script = r#"
            void main() {
                int x = 5;
                int y = 10;
                int result = x + y;
            }
        "#;

        module.add_script_section("main", valid_script, 0)?;
        let build_result = module.build();

        // Check that compilation succeeded
        assert!(
            build_result.is_ok(),
            "Valid script should compile successfully"
        );

        // Check diagnostics - should be empty or only contain info/warnings
        assert!(
            !diagnostics.has_errors(),
            "Valid script should not have errors"
        );

        // Print diagnostics if any (for debugging)
        if !diagnostics.is_empty() {
            println!("Diagnostics for valid script:");
            println!("{}", diagnostics);
        }

        Ok(())
    }

    #[test]
    fn test_diagnostics_with_compilation_errors() -> ScriptResult<()> {
        // Create the script engine
        let mut engine = Engine::create().expect("Failed to create script engine");
        let mut diagnostics = Diagnostics::new();

        // Set up diagnostic callback
        engine.set_diagnostic_callback(&mut diagnostics)?;

        // Install string addon
        // Create a module with invalid script
        let module = engine.get_module("ErrorTestModule", GetModuleFlags::AlwaysCreate)?;

        let invalid_script = r#"
            void main() {
                // This should cause an error - undefined function
                undefined_function();

                // This should cause another error - undefined variable
                int result = undefined_variable + 5;

                // Syntax error - missing semicolon
                int x = 5
            }
        "#;

        module.add_script_section("error_script", invalid_script, 0)?;
        let build_result = module.build();

        // Check that compilation failed
        assert!(
            build_result.is_err(),
            "Invalid script should fail to compile"
        );

        // Check diagnostics
        assert!(
            diagnostics.has_errors(),
            "Invalid script should have errors"
        );
        assert!(
            diagnostics.error_count() > 0,
            "Should have at least one error"
        );

        println!("Compilation errors found: {}", diagnostics.error_count());
        println!("Total diagnostics: {}", diagnostics.count());

        // Print all diagnostics
        println!("All diagnostics:");
        println!("{}", diagnostics);

        // Print only errors
        println!("Errors only:");
        for error in diagnostics.errors() {
            println!("  {}", error);
        }

        Ok(())
    }

    #[test]
    fn test_diagnostics_with_warnings() -> ScriptResult<()> {
        // Create the script engine
        let mut engine = Engine::create().expect("Failed to create script engine");
        let mut diagnostics = Diagnostics::new();

        // Set up diagnostic callback
        engine.set_diagnostic_callback(&mut diagnostics)?;

        // Create a module with script that might generate warnings
        let module = engine.get_module("WarningTestModule", GetModuleFlags::AlwaysCreate)?;

        let warning_script = r#"
            void main() {
                // This might generate a warning - unused variable
                int unused_variable = 42;
            }
        "#;

        module.add_script_section("warning_script", warning_script, 0)?;
        let build_result = module.build();

        // Check that compilation succeeded
        assert!(
            build_result.is_ok(),
            "Script with warnings should still compile"
        );

        // Print diagnostics to see what we get
        println!("Diagnostics for warning script:");
        println!("Has errors: {}", diagnostics.has_errors());
        println!("Has warnings: {}", diagnostics.has_warnings());
        println!("Error count: {}", diagnostics.error_count());
        println!("Warning count: {}", diagnostics.warning_count());
        println!("Total count: {}", diagnostics.count());

        if !diagnostics.is_empty() {
            println!("All diagnostics:");
            println!("{}", diagnostics);
        }

        Ok(())
    }

    #[test]
    fn test_diagnostics_clear_and_reuse() -> ScriptResult<()> {
        // Create the script engine
        let mut engine = Engine::create().expect("Failed to create script engine");
        let mut diagnostics = Diagnostics::new();

        // Set up diagnostic callback
        engine.set_diagnostic_callback(&mut diagnostics)?;

        // First compilation with errors
        let module1 = engine.get_module("TestModule1", GetModuleFlags::AlwaysCreate)?;
        module1.add_script_section("script1", "void main() { undefined_function(); }", 0)?;
        let _ = module1.build(); // Ignore result

        assert!(
            diagnostics.has_errors(),
            "First compilation should have errors"
        );
        let first_error_count = diagnostics.error_count();
        assert!(first_error_count > 0, "Should have at least one error");

        // Clear diagnostics
        diagnostics.clear();
        assert!(
            diagnostics.is_empty(),
            "Diagnostics should be empty after clear"
        );
        assert!(
            !diagnostics.has_errors(),
            "Should not have errors after clear"
        );
        assert_eq!(diagnostics.count(), 0, "Count should be zero after clear");

        // Second compilation with valid script
        let module2 = engine.get_module("TestModule2", GetModuleFlags::AlwaysCreate)?;
        module2.add_script_section("script2", "void main() { int a = 0; }", 0)?;
        let build_result = module2.build();

        assert!(build_result.is_ok(), "Second compilation should succeed");
        assert!(
            !diagnostics.has_errors(),
            "Second compilation should not have errors"
        );

        println!("✅ Diagnostics clear and reuse test passed");

        Ok(())
    }

    #[test]
    fn test_diagnostics_multiple_sections() -> ScriptResult<()> {
        // Create the script engine
        let mut engine = Engine::create().expect("Failed to create script engine");
        let mut diagnostics = Diagnostics::new();

        // Set up diagnostic callback
        engine.set_diagnostic_callback(&mut diagnostics)?;

        // Create a module with multiple sections
        let module = engine.get_module("MultiSectionModule", GetModuleFlags::AlwaysCreate)?;

        // Add multiple script sections with different issues
        module.add_script_section(
            "section1",
            r#"
            void function1() {
                undefined_function_1(); // Error in section1
            }
        "#,
            0,
        )?;

        module.add_script_section(
            "section2",
            r#"
            void function2() {
                undefined_function_2(); // Error in section2
            }
        "#,
            0,
        )?;

        module.add_script_section(
            "section3",
            r#"
            void main() {
                int a = 3;
            }
        "#,
            0,
        )?;

        let build_result = module.build();

        assert!(
            build_result.is_err(),
            "Module with errors should fail to build"
        );
        assert!(
            diagnostics.has_errors(),
            "Should have errors from multiple sections"
        );

        println!("Diagnostics from multiple sections:");
        for diagnostic in diagnostics.iter() {
            println!(
                "  Section: {:?}, Row: {}, Col: {}, Kind: {:?}, Message: {}",
                diagnostic.section,
                diagnostic.row,
                diagnostic.col,
                diagnostic.kind,
                diagnostic.message
            );
        }

        // Check that we have errors from different sections
        let sections_with_errors: std::collections::HashSet<_> = diagnostics
            .errors()
            .filter_map(|d| d.section.as_ref())
            .collect();

        println!("Sections with errors: {:?}", sections_with_errors);

        Ok(())
    }

    #[test]
    fn test_diagnostic_display_formatting() {
        let diagnostic_with_section = Diagnostic {
            kind: DiagnosticKind::Error,
            message: "Undefined symbol 'foo'".to_string(),
            section: Some("main.as".to_string()),
            row: 10,
            col: 5,
        };

        let diagnostic_without_section = Diagnostic {
            kind: DiagnosticKind::Warning,
            message: "Unused variable 'x'".to_string(),
            section: None,
            row: 15,
            col: 8,
        };

        assert_eq!(
            diagnostic_with_section.to_string(),
            "main.as:10:5: error: Undefined symbol 'foo'"
        );

        assert_eq!(
            diagnostic_without_section.to_string(),
            "15:8: warning: Unused variable 'x'"
        );

        let mut diagnostics = Diagnostics::new();
        diagnostics.add_diagnostic(diagnostic_with_section);
        diagnostics.add_diagnostic(diagnostic_without_section);

        let formatted = diagnostics.to_string();
        assert!(formatted.contains("main.as:10:5: error: Undefined symbol 'foo'"));
        assert!(formatted.contains("15:8: warning: Unused variable 'x'"));

        println!("Formatted diagnostics:\n{}", formatted);
    }

    #[test]
    fn test_diagnostic_iterators() {
        let mut diagnostics = Diagnostics::new();

        // Add various types of diagnostics
        diagnostics.add_diagnostic(Diagnostic {
            kind: DiagnosticKind::Error,
            message: "Error 1".to_string(),
            section: None,
            row: 1,
            col: 1,
        });

        diagnostics.add_diagnostic(Diagnostic {
            kind: DiagnosticKind::Warning,
            message: "Warning 1".to_string(),
            section: None,
            row: 2,
            col: 1,
        });

        diagnostics.add_diagnostic(Diagnostic {
            kind: DiagnosticKind::Error,
            message: "Error 2".to_string(),
            section: None,
            row: 3,
            col: 1,
        });

        diagnostics.add_diagnostic(Diagnostic {
            kind: DiagnosticKind::Info,
            message: "Info 1".to_string(),
            section: None,
            row: 4,
            col: 1,
        });

        // Test counts
        assert_eq!(diagnostics.count(), 4);
        assert_eq!(diagnostics.error_count(), 2);
        assert_eq!(diagnostics.warning_count(), 1);
        assert_eq!(diagnostics.info_count(), 1);

        // Test iterators
        let errors: Vec<_> = diagnostics.errors().collect();
        assert_eq!(errors.len(), 2);
        assert!(errors.iter().all(|d| d.kind == DiagnosticKind::Error));

        let warnings: Vec<_> = diagnostics.warnings().collect();
        assert_eq!(warnings.len(), 1);
        assert!(warnings.iter().all(|d| d.kind == DiagnosticKind::Warning));

        // Test flags
        assert!(diagnostics.has_errors());
        assert!(diagnostics.has_warnings());
        assert!(!diagnostics.is_empty());
    }
}
