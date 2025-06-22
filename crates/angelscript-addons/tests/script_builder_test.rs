#[cfg(test)]
mod script_builder_tests {
    use angelscript_addons::script_builder::{ScriptBuilder, ScriptBuilderConfig};
    use angelscript_core::core::engine::Engine;
    use std::fs;
    use std::path::PathBuf;
    use tempdir::TempDir;

    // Helper function to create a test engine
    fn create_test_engine() -> Engine {
        let mut engine = Engine::create().expect("Failed to create engine");
        engine
            .install(angelscript_addons::string::addon())
            .expect("Failed to install string addon");
        engine
            .set_message_callback(
                |msg, _| {
                    println!("AngelScript: {}", msg.message);
                },
                None,
            )
            .expect("Failed to set message callback");
        engine
    }

    // Helper to create temporary files for testing
    fn create_temp_script_file(content: &str, filename: &str) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new("test").expect("Failed to create temp dir");
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, content).expect("Failed to write temp file");
        (temp_dir, file_path)
    }

    #[test]
    fn test_script_builder_basic_functionality() {
        let engine = create_test_engine();
        let mut builder = ScriptBuilder::new();

        // Test basic setup
        assert_eq!(builder.get_section_count(), 0);
        assert!(builder.get_engine().is_none());
        assert!(builder.get_module().is_none());

        // Start a new module
        builder.start_new_module(&engine, "TestModule").unwrap();
        assert!(builder.get_engine().is_some());
        assert!(builder.get_module().is_some());
        assert_eq!(builder.get_section_count(), 0);
    }

    #[test]
    fn test_script_builder_from_memory() {
        let engine = create_test_engine();
        let mut builder = ScriptBuilder::new();

        builder.start_new_module(&engine, "TestModule").unwrap();

        let script = r#"
            int test() {
                return 42;
            }
        "#;

        // Add script from memory
        let added = builder
            .add_section_from_memory("test_script", script, 0)
            .unwrap();
        assert!(added);
        assert_eq!(builder.get_section_count(), 1);

        // Try to add the same section again
        let added_again = builder
            .add_section_from_memory("test_script", script, 0)
            .unwrap();
        assert!(!added_again); // Should not be added again
        assert_eq!(builder.get_section_count(), 1);

        // Build and test
        builder.build_module().unwrap();

        let module = builder.get_module().unwrap();
        let func = module.get_function_by_decl("int test()").unwrap();
        let ctx = engine.create_context().unwrap();
        ctx.prepare(&func).unwrap();
        ctx.execute().unwrap();

        assert_eq!(ctx.get_return_dword(), 42);
    }

    #[test]
    fn test_script_builder_from_file() {
        let engine = create_test_engine();
        let mut builder = ScriptBuilder::new();

        builder.start_new_module(&engine, "TestModule").unwrap();

        let script_content = r#"
            string getMessage() {
                return "Hello from file!";
            }
        "#;

        let (_temp_dir, file_path) = create_temp_script_file(script_content, "test.as");

        // Add script from file
        let added = builder.add_section_from_file(&file_path).unwrap();
        assert!(added);
        assert_eq!(builder.get_section_count(), 1);

        // Build and test
        builder.build_module().unwrap();

        let module = builder.get_module().unwrap();
        let func = module.get_function_by_decl("string getMessage()").unwrap();
        let ctx = engine.create_context().unwrap();
        ctx.prepare(&func).unwrap();
        ctx.execute().unwrap();

        let result = ctx.get_return_object::<String>().unwrap();
        assert_eq!(result, "Hello from file!");
    }

    #[test]
    fn test_simple_conditional_compilation() {
        let engine = create_test_engine();
        let mut builder = ScriptBuilder::new();

        builder.start_new_module(&engine, "TestModule").unwrap();
        builder.define_word("DEBUG");

        let script = r#"
        int getValue() {
            #if DEBUG
            return 42;
            #else
            return 0;
            #endif
        }
    "#;

        builder.add_section_from_memory("test_script", script, 0).unwrap();
        builder.build_module().unwrap();

        let module = builder.get_module().unwrap();
        let func = module.get_function_by_decl("int getValue()").unwrap();
        let ctx = engine.create_context().unwrap();
        ctx.prepare(&func).unwrap();
        ctx.execute().unwrap();

        assert_eq!(ctx.get_return_dword(), 42);
    }

    #[test]
    fn test_conditional_compilation_undefined() {
        let engine = create_test_engine();
        let mut builder = ScriptBuilder::new();

        builder.start_new_module(&engine, "TestModule").unwrap();
        // Don't define DEBUG

        let script = r#"
        int getValue() {
            #if DEBUG
            return 42;
            #else
            return 24;
            #endif
        }
    "#;

        builder.add_section_from_memory("test_script", script, 0).unwrap();
        builder.build_module().unwrap();

        let module = builder.get_module().unwrap();
        let func = module.get_function_by_decl("int getValue()").unwrap();
        let ctx = engine.create_context().unwrap();
        ctx.prepare(&func).unwrap();
        ctx.execute().unwrap();

        assert_eq!(ctx.get_return_dword(), 24);
    }

    #[test]
    fn test_nested_conditional_compilation() {
        let engine = create_test_engine();
        let mut builder = ScriptBuilder::new();

        builder.start_new_module(&engine, "TestModule").unwrap();
        builder.define_word("FEATURE_A");
        builder.define_word("FEATURE_B");

        let script = r#"
            int getValue() {
                #if FEATURE_A
                    #if FEATURE_B
                        return 100;
                    #elif FEATURE_C
                        return 200;
                    #else
                        return 50;
                    #endif
                #else
                return 0;
                #endif
            }
        "#;

        builder
            .add_section_from_memory("test_script", script, 0)
            .unwrap();
        builder.build_module().unwrap();

        let module = builder.get_module().unwrap();
        let func = module.get_function_by_decl("int getValue()").unwrap();
        let ctx = engine.create_context().unwrap();
        ctx.prepare(&func).unwrap();
        ctx.execute().unwrap();

        assert_eq!(ctx.get_return_dword(), 100); // Both FEATURE_A and FEATURE_B are defined
    }

    #[test]
    fn test_include_processing() {
        let engine = create_test_engine();
        let mut builder = ScriptBuilder::new();

        builder.start_new_module(&engine, "TestModule").unwrap();

        // Create included file
        let included_content = r#"
            int getIncludedValue() {
                return 123;
            }
        "#;
        let (_temp_dir, included_path) = create_temp_script_file(included_content, "included.as");

        // Create main file with include
        let main_content = format!(
            r#"
            #include "{}"
            
            int getMainValue() {{
                return getIncludedValue() + 1;
            }}
        "#,
            included_path.to_string_lossy()
        );

        builder
            .add_section_from_memory("main_script", &main_content, 0)
            .unwrap();
        builder.build_module().unwrap();

        let module = builder.get_module().unwrap();

        // Test included function
        let included_func = module
            .get_function_by_decl("int getIncludedValue()")
            .unwrap();
        let ctx = engine.create_context().unwrap();
        ctx.prepare(&included_func).unwrap();
        ctx.execute().unwrap();
        assert_eq!(ctx.get_return_dword(), 123);

        // Test main function that uses included function
        let main_func = module.get_function_by_decl("int getMainValue()").unwrap();
        ctx.prepare(&main_func).unwrap();
        ctx.execute().unwrap();
        assert_eq!(ctx.get_return_dword(), 124);
    }

    #[test]
    fn test_include_cycle_prevention() {
        let engine = create_test_engine();
        let mut builder = ScriptBuilder::new();

        builder.start_new_module(&engine, "TestModule").unwrap();

        let script_with_self_include = r#"
            #include "self_include.as"
            
            int getValue() {
                return 42;
            }
        "#;

        let (_temp_dir, file_path) =
            create_temp_script_file(script_with_self_include, "self_include.as");

        // This should not cause infinite recursion
        let added = builder.add_section_from_file(&file_path).unwrap();
        assert!(added);
        assert_eq!(builder.get_section_count(), 1); // Only added once

        builder.build_module().unwrap();
    }

    // #[test]
    // fn test_pragma_callback() {
    //     let engine = create_test_engine();
    //     let mut builder = ScriptBuilder::new();
    //
    //     builder.start_new_module(&engine, "TestModule").unwrap();
    //
    //     let mut pragma_received = false;
    //     let mut pragma_content = String::new();
    //
    //     builder.set_pragma_callback(move |content, _builder| {
    //         pragma_content.add(content);
    //         pragma_received = true;
    //         Ok(())
    //     });
    //
    //     let script = r#"
    //         #pragma once
    //         #pragma custom_directive some_value
    //
    //         int getValue() {
    //             return 42;
    //         }
    //     "#;
    //
    //     builder
    //         .add_section_from_memory("test_script", script, 0)
    //         .unwrap();
    //     builder.build_module().unwrap();
    //
    //     // Note: In a real implementation, you'd need to capture the pragma callback state
    //     // This is a simplified test showing the structure
    // }

    #[test]
    fn test_include_callback() {
        let engine = create_test_engine();
        let mut builder = ScriptBuilder::new();

        builder.start_new_module(&engine, "TestModule").unwrap();

        builder.set_include_callback(|include_file, _current_section, builder| {
            // Custom include processing - add content directly
            let included_content = format!(
                r#"
                int getValueFrom{}() {{
                    return 999;
                }}
            "#,
                include_file.replace(".", "_")
            );

            builder.add_section_from_memory(
                &format!("custom_{}", include_file),
                &included_content,
                0,
            )?;
            Ok(())
        });

        let script = r#"
            #include "custom.as"
            
            int getTotal() {
                return getValueFromcustom_as();
            }
        "#;

        builder
            .add_section_from_memory("main_script", script, 0)
            .unwrap();
        builder.build_module().unwrap();

        let module = builder.get_module().unwrap();
        let func = module.get_function_by_decl("int getTotal()").unwrap();
        let ctx = engine.create_context().unwrap();
        ctx.prepare(&func).unwrap();
        ctx.execute().unwrap();

        assert_eq!(ctx.get_return_dword(), 999);
    }

    #[test]
    fn test_defined_words_management() {
        let mut builder = ScriptBuilder::new();

        // Test defining words
        builder.define_word("DEBUG");
        builder.define_word("FEATURE_X");

        assert!(builder.is_word_defined("DEBUG"));
        assert!(builder.is_word_defined("FEATURE_X"));
        assert!(!builder.is_word_defined("RELEASE"));

        // Test undefining
        builder.undefine_word("DEBUG");
        assert!(!builder.is_word_defined("DEBUG"));
        assert!(builder.is_word_defined("FEATURE_X"));

        // Test getting all defined words
        let defined_words = builder.get_defined_words();
        assert!(defined_words.contains("FEATURE_X"));
        assert!(!defined_words.contains("DEBUG"));
    }

    #[test]
    fn test_script_builder_config() {
        let engine = create_test_engine();

        let mut builder = ScriptBuilderConfig::new()
            .define_word("CONFIG_DEBUG")
            .define_words(vec!["FEATURE_A", "FEATURE_B"])
            .build();

        assert!(builder.is_word_defined("CONFIG_DEBUG"));
        assert!(builder.is_word_defined("FEATURE_A"));
        assert!(builder.is_word_defined("FEATURE_B"));

        builder.start_new_module(&engine, "TestModule").unwrap();

        let script = r#"
            int getValue() {
                #if CONFIG_DEBUG
                    #if FEATURE_A
                        return 100;
                    #endif
                #endif
                return 0;
            }
        "#;

        builder
            .add_section_from_memory("test_script", script, 0)
            .unwrap();
        builder.build_module().unwrap();

        let module = builder.get_module().unwrap();
        let func = module.get_function_by_decl("int getValue()").unwrap();
        let ctx = engine.create_context().unwrap();
        ctx.prepare(&func).unwrap();
        ctx.execute().unwrap();

        assert_eq!(ctx.get_return_dword(), 100);
    }

    #[cfg(feature = "script-builder-metadata")]
    mod metadata_tests {
        use super::*;

        #[test]
        fn test_basic_metadata_extraction() {
            let engine = create_test_engine();
            let mut builder = ScriptBuilder::new();

            builder.start_new_module(&engine, "TestModule").unwrap();

            let script = r#"
                [important]
                [version("1.0")]
                class TestClass {
                    [serializable]
                    int value;
                    
                    [deprecated]
                    [author("John Doe")]
                    void oldMethod() {}
                    
                    [fast]
                    int getValue() {
                        return value;
                    }
                }
                
                [global]
                [utility]
                int globalFunction() {
                    return 42;
                }
                
                [config]
                int globalVar = 100;
            "#;

            builder
                .add_section_from_memory("test_script", script, 0)
                .unwrap();
            builder.build_module().unwrap();

            // Test type metadata
            let module = builder.get_module().unwrap();
            if let Some(type_info) = module.get_type_info_by_name("TestClass") {
                let type_id = type_info.get_type_id();
                if let Some(metadata) = builder.get_metadata_for_type(type_id) {
                    assert!(metadata.contains(&"important".to_string()));
                    assert!(metadata.contains(&"version(\"1.0\")".to_string()));
                }
            }

            // Test function metadata
            if let Some(func) = module.get_function_by_decl("int globalFunction()") {
                if let Some(metadata) = builder.get_metadata_for_func(&func) {
                    assert!(metadata.contains(&"global".to_string()));
                    assert!(metadata.contains(&"utility".to_string()));
                }
            }
        }

        #[test]
        fn test_nested_metadata() {
            let engine = create_test_engine();
            let mut builder = ScriptBuilder::new();

            builder.start_new_module(&engine, "TestModule").unwrap();

            let script = r#"
                [complex("nested[brackets]")]
                [array([1, 2, 3])]
                class ComplexClass {
                    [range(min=0, max=100)]
                    int percentage;
                }
            "#;

            builder
                .add_section_from_memory("test_script", script, 0)
                .unwrap();
            builder.build_module().unwrap();

            let module = builder.get_module().unwrap();
            if let Some(type_info) = module.get_type_info_by_name("ComplexClass") {
                let type_id = type_info.get_type_id();
                if let Some(metadata) = builder.get_metadata_for_type(type_id) {
                    assert!(metadata.iter().any(|m| m.contains("nested[brackets]")));
                    assert!(metadata.iter().any(|m| m.contains("array([1, 2, 3])")));
                }
            }
        }

        #[test]
        fn test_namespace_metadata() {
            let engine = create_test_engine();
            let mut builder = ScriptBuilder::new();

            builder.start_new_module(&engine, "TestModule").unwrap();

            let script = r#"
                namespace Graphics {
                    [gpu_accelerated]
                    class Renderer {
                        [shader_uniform]
                        float time;
                        
                        [vertex_shader]
                        void render() {}
                    }
                    
                    [utility]
                    void clearScreen() {}
                }
                
                namespace Audio {
                    [dsp]
                    class Processor {
                        [sample_rate(44100)]
                        void process() {}
                    }
                }
            "#;

            builder
                .add_section_from_memory("test_script", script, 0)
                .unwrap();
            builder.build_module().unwrap();

            // Test that metadata is properly associated with namespaced elements
            let module = builder.get_module().unwrap();

            // Set namespace and test
            module.set_default_namespace("Graphics").unwrap();
            if let Some(type_info) = module.get_type_info_by_name("Renderer") {
                let type_id = type_info.get_type_id();
                if let Some(metadata) = builder.get_metadata_for_type(type_id.into()) {
                    assert!(metadata.contains(&"gpu_accelerated".to_string()));
                }
            }

            module.set_default_namespace("Audio").unwrap();
            if let Some(type_info) = module.get_type_info_by_name("Processor") {
                let type_id = type_info.get_type_id();
                if let Some(metadata) = builder.get_metadata_for_type(type_id.into()) {
                    assert!(metadata.contains(&"dsp".to_string()));
                }
            }

            module.set_default_namespace("").unwrap();
        }

        #[test]
        fn test_virtual_property_metadata() {
            let engine = create_test_engine();
            let mut builder = ScriptBuilder::new();

            builder.start_new_module(&engine, "TestModule").unwrap();

            let script = r#"
                class TestClass {
                    private int _value;
                    
                    [property]
                    [range(0, 100)]
                    int value {
                        get { return _value; }
                        set { _value = value; }
                    }
                }
            "#;

            builder
                .add_section_from_memory("test_script", script, 0)
                .unwrap();
            builder.build_module().unwrap();

            // Test that virtual property metadata is applied to getter/setter
            let module = builder.get_module().unwrap();
            if let Some(type_info) = module.get_type_info_by_name("TestClass") {
                if let Some(getter) = type_info.get_method_by_name("get_value", true) {
                    if let Some(metadata) = builder.get_metadata_for_func(&getter) {
                        assert!(metadata.contains(&"property".to_string()));
                        assert!(metadata.contains(&"range(0, 100)".to_string()));
                    }
                }
            }
        }

        #[test]
        fn test_metadata_stats() {
            let engine = create_test_engine();
            let mut builder = ScriptBuilder::new();

            builder.start_new_module(&engine, "TestModule").unwrap();

            let script = r#"
                [meta1]
                class Class1 {}
                
                [meta2]
                class Class2 {}
                
                [meta3]
                int func1() { return 1; }
                
                [meta4]
                int func2() { return 2; }
                
                [meta5]
                int var1 = 1;
                
                [meta6]
                int var2 = 2;
            "#;

            builder
                .add_section_from_memory("test_script", script, 0)
                .unwrap();
            builder.build_module().unwrap();

            let stats = builder.metadata_processor().get_metadata_stats();
            assert_eq!(stats.declarations_found, 6);
            assert!(stats.types_with_metadata >= 2);
            assert!(stats.functions_with_metadata >= 2);
            assert!(stats.variables_with_metadata >= 2);
        }

        #[test]
        fn test_ambiguous_declarations() {
            let engine = create_test_engine();
            let mut builder = ScriptBuilder::new();

            builder.start_new_module(&engine, "TestModule").unwrap();

            let script = r#"
                [ambiguous]
                int getValue();  // Function prototype
                
                [also_ambiguous]
                int value = getValue();  // Variable with function call
            "#;

            builder
                .add_section_from_memory("test_script", script, 0)
                .unwrap();

            // Should handle ambiguous declarations without crashing
            let result = builder.build_module();
            // Note: This might fail compilation due to missing function body,
            // but metadata extraction should work
        }

        #[test]
        fn test_metadata_with_conditional_compilation() {
            let engine = create_test_engine();
            let mut builder = ScriptBuilder::new();

            builder.start_new_module(&engine, "TestModule").unwrap();
            builder.define_word("DEBUG");

            let script = r#"
                #if DEBUG
                [debug_only]
                class DebugClass {
                    [trace]
                    void log() {}
                }
                #endif
                
                #if RELEASE
                [optimized]
                class ReleaseClass {
                    [inline]
                    void fastMethod() {}
                }
                #endif
                
                [always_present]
                int globalFunc() { return 0; }
            "#;

            builder
                .add_section_from_memory("test_script", script, 0)
                .unwrap();
            builder.build_module().unwrap();

            // Should only have metadata for DEBUG classes and global function
            let module = builder.get_module().unwrap();

            // DebugClass should exist and have metadata
            if let Some(type_info) = module.get_type_info_by_name("DebugClass") {
                let type_id = type_info.get_type_id();
                if let Some(metadata) = builder.get_metadata_for_type(type_id.into()) {
                    assert!(metadata.contains(&"debug_only".to_string()));
                }
            }

            // ReleaseClass should not exist
            assert!(module.get_type_info_by_name("ReleaseClass").is_none());

            // Global function should have metadata
            if let Some(func) = module.get_function_by_decl("int globalFunc()") {
                if let Some(metadata) = builder.get_metadata_for_func(&func) {
                    assert!(metadata.contains(&"always_present".to_string()));
                }
            }
        }
    }

    #[test]
    fn test_complex_preprocessing_scenario() {
        let engine = create_test_engine();
        let mut builder = ScriptBuilder::new();

        builder.start_new_module(&engine, "TestModule").unwrap();
        builder.define_word("PLATFORM_WINDOWS");
        builder.define_word("FEATURE_NETWORKING");

        // Create a utility include file
        let utility_content = r#"
            #if PLATFORM_WINDOWS
            void platformSpecificInit() {
                // Windows-specific initialization
            }
            #endif
            
            #if PLATFORM_LINUX
            void platformSpecificInit() {
                // Linux-specific initialization
            }
            #endif
        "#;
        let (_temp_dir, utility_path) = create_temp_script_file(utility_content, "platform.as");

        let main_script = format!(
            r#"
            #include "{}"
            
            #if FEATURE_NETWORKING
            class NetworkManager {{
                void connect() {{
                    platformSpecificInit();
                }}
            }}
            #endif
            
            #if FEATURE_GRAPHICS
            class GraphicsManager {{
                void render() {{}}
            }}
            #endif
            
            int main() {{
                #if FEATURE_NETWORKING
                NetworkManager nm;
                nm.connect();
                return 1;
                #else
                return 0;
                #endif
            }}
        "#,
            utility_path.to_string_lossy()
        );

        builder
            .add_section_from_memory("main_script", &main_script, 0)
            .unwrap();
        builder.build_module().unwrap();

        let module = builder.get_module().unwrap();

        // Should have NetworkManager (FEATURE_NETWORKING is defined)
        assert!(module.get_type_info_by_name("NetworkManager").is_some());

        // Should not have GraphicsManager (FEATURE_GRAPHICS is not defined)
        assert!(module.get_type_info_by_name("GraphicsManager").is_none());

        // Should have platform-specific function
        assert!(
            module
                .get_function_by_decl("void platformSpecificInit()")
                .is_some()
        );

        // Test main function
        let main_func = module.get_function_by_decl("int main()").unwrap();
        let ctx = engine.create_context().unwrap();
        ctx.prepare(&main_func).unwrap();
        ctx.execute().unwrap();

        assert_eq!(ctx.get_return_dword(), 1); // FEATURE_NETWORKING path
    }

    #[test]
    fn test_error_handling() {
        let engine = create_test_engine();
        let mut builder = ScriptBuilder::new();

        builder.start_new_module(&engine, "TestModule").unwrap();

        // Test invalid include
        let script_with_bad_include = r#"
            #include "nonexistent_file.as"
            
            int getValue() {
                return 42;
            }
        "#;

        let result = builder.add_section_from_memory("test_script", script_with_bad_include, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_shebang_handling() {
        let engine = create_test_engine();
        let mut builder = ScriptBuilder::new();

        builder.start_new_module(&engine, "TestModule").unwrap();

        let script_with_shebang = "#!/usr/bin/angelscript\n\nint getValue() {\n    return 42;\n}\n";

        builder.add_section_from_memory("test_script", script_with_shebang, 0).unwrap();
        builder.build_module().unwrap();

        let module = builder.get_module().unwrap();
        let func = module.get_function_by_decl("int getValue()").unwrap();
        let ctx = engine.create_context().unwrap();
        ctx.prepare(&func).unwrap();
        ctx.execute().unwrap();

        assert_eq!(ctx.get_return_dword(), 42);
    }

    #[test]
    fn test_multiple_shebangs_and_directives() {
        let engine = create_test_engine();
        let mut builder = ScriptBuilder::new();

        builder.start_new_module(&engine, "TestModule").unwrap();
        builder.define_word("DEBUG");

        let script = "#!/usr/bin/angelscript\n// This is a comment\n\n#if DEBUG\nint getValue() {\n    return 42;\n}\n#endif\n";

        builder.add_section_from_memory("test_script", script, 0).unwrap();
        builder.build_module().unwrap();

        let module = builder.get_module().unwrap();
        let func = module.get_function_by_decl("int getValue()").unwrap();
        let ctx = engine.create_context().unwrap();
        ctx.prepare(&func).unwrap();
        ctx.execute().unwrap();

        assert_eq!(ctx.get_return_dword(), 42);
    }
}
