#[cfg(test)]
mod tests {
    use angelscript::core::engine::Engine;
    use angelscript::prelude::{ContextState, GetModuleFlags};

    #[test]
    fn test_op_assign() {
        let script = r#"
            string test() {
                string a = "Hello";
                string b = "World";
                a = b;
                return a;
            }
        "#;

        let mut engine = Engine::create().expect("Failed to create engine");
        engine
            .with_default_modules()
            .expect("Failed to register std");
        engine
            .set_message_callback(|msg| {
                println!("AngelScript: {}", msg.message);
            })
            .expect("Failed to set message callback");

        // Create or reuse a module for the test script
        let module = engine
            .get_module("TestModule", GetModuleFlags::CreateIfNotExists)
            .expect("Failed to get module");
        module
            .add_script_section_simple("test_script", script)
            .expect("Failed to add script section");
        module.build().expect("Failed to build module");

        let func = module
            .get_function_by_decl("string test()")
            .expect("Failed to get function");
        let ctx = engine.create_context().expect("Failed to create context");
        ctx.prepare(&func).expect("Failed to prepare context");
        ctx.execute().expect("Failed to execute script");

        let result = ctx.get_return_object::<String>();
        assert_eq!(result.unwrap(), "World");
    }

    #[test]
    fn test_op_add_assign() {
        let script = r#"
            string test() {
                string a = "Hello";
                a += " World";
                return a;
            }
        "#;

        let mut engine = Engine::create().expect("Failed to create engine");
        engine
            .with_default_modules()
            .expect("Failed to register std");
        engine
            .set_message_callback(|msg| {
                println!("AngelScript: {}", msg.message);
            })
            .expect("Failed to set message callback");

        // Create or reuse a module for the test script
        let module = engine
            .get_module("TestModule", GetModuleFlags::CreateIfNotExists)
            .expect("Failed to get module");
        module
            .add_script_section_simple("test_script", script)
            .expect("Failed to add script section");
        module.build().expect("Failed to build module");

        let func = module
            .get_function_by_decl("string test()")
            .expect("Failed to get function");
        let ctx = engine.create_context().expect("Failed to create context");
        ctx.prepare(&func).expect("Failed to prepare context");
        ctx.execute().expect("Failed to execute script");

        let result = ctx.get_return_object::<String>();
        assert_eq!(result.unwrap(), "Hello World");
    }

    #[test]
    fn test_op_equals() {
        let script = r#"
            bool equal() {
                string a = "test";
                string b = "test";
                return a == b;
            }

            bool not_equal() {
                string a = "test";
                string b = "different";
                return a == b;
            }
        "#;

        let mut engine = Engine::create().expect("Failed to create engine");
        engine
            .with_default_modules()
            .expect("Failed to register std");
        engine
            .set_message_callback(|msg| {
                println!("AngelScript: {}", msg.message);
            })
            .expect("Failed to set message callback");

        // Create or reuse a module for the test script
        let module = engine
            .get_module("TestModule", GetModuleFlags::CreateIfNotExists)
            .expect("Failed to get module");
        module
            .add_script_section_simple("test_script", script)
            .expect("Failed to add script section");
        module.build().expect("Failed to build module");

        let func = module
            .get_function_by_decl("bool equal()")
            .expect("Failed to get function");
        let ctx = engine.create_context().expect("Failed to create context");
        ctx.prepare(&func).expect("Failed to prepare context");
        ctx.execute().expect("Failed to execute script");

        let result = ctx.get_address_of_return_value::<bool>();
        assert!(result.unwrap());

        let func = module
            .get_function_by_decl("bool not_equal()")
            .expect("Failed to get function");
        let ctx = engine.create_context().expect("Failed to create context");
        ctx.prepare(&func).expect("Failed to prepare context");
        ctx.execute().expect("Failed to execute script");

        let result = ctx.get_address_of_return_value::<bool>();
        assert!(!result.unwrap());
    }

    #[test]
    fn test_op_cmp() {
        let script = r#"
            bool lt() {
                string a = "abc";
                string b = "bcd";
                return a < b;
            }
            bool gt() {
                string a = "abc";
                string b = "bcd";
                return b > a;
            }
        "#;

        let mut engine = Engine::create().expect("Failed to create engine");
        engine
            .with_default_modules()
            .expect("Failed to register std");
        engine
            .set_message_callback(|msg| {
                println!("AngelScript: {}", msg.message);
            })
            .expect("Failed to set message callback");

        // Create or reuse a module for the test script
        let module = engine
            .get_module("TestModule", GetModuleFlags::CreateIfNotExists)
            .expect("Failed to get module");
        module
            .add_script_section_simple("test_script", script)
            .expect("Failed to add script section");
        module.build().expect("Failed to build module");

        let func = module
            .get_function_by_decl("bool lt()")
            .expect("Failed to get function");
        let ctx = engine.create_context().expect("Failed to create context");
        ctx.prepare(&func).expect("Failed to prepare context");
        ctx.execute().expect("Failed to execute script");

        let result = ctx.get_address_of_return_value::<bool>();
        assert!(result.unwrap());

        let func = module
            .get_function_by_decl("bool gt()")
            .expect("Failed to get function");
        let ctx = engine.create_context().expect("Failed to create context");
        ctx.prepare(&func).expect("Failed to prepare context");
        ctx.execute().expect("Failed to execute script");

        let result = ctx.get_address_of_return_value::<bool>();
        assert!(result.unwrap());
    }

    #[test]
    fn test_string_length() {
        let script = r#"
            uint test() {
                string a = "Hello";
                return a.length();
            }
        "#;

        let mut engine = Engine::create().expect("Failed to create engine");
        engine
            .with_default_modules()
            .expect("Failed to register std");
        engine
            .set_message_callback(|msg| {
                println!("AngelScript: {}", msg.message);
            })
            .expect("Failed to set message callback");

        // Create or reuse a module for the test script
        let module = engine
            .get_module("TestModule", GetModuleFlags::CreateIfNotExists)
            .expect("Failed to get module");
        module
            .add_script_section_simple("test_script", script)
            .expect("Failed to add script section");
        module.build().expect("Failed to build module");

        let func = module
            .get_function_by_decl("uint test()")
            .expect("Failed to get function");
        let ctx = engine.create_context().expect("Failed to create context");
        ctx.prepare(&func).expect("Failed to prepare context");
        ctx.execute().expect("Failed to execute script");

        let result = ctx.get_return_dword();
        assert_eq!(result, 5);
    }

    #[test]
    fn test_string_is_empty() {
        let script = r#"
            bool not_empty() {
                string a = "Hello";
                return a.isEmpty();
            }
            bool empty() {
                string b = "";
                return b.isEmpty();
            }
        "#;

        let mut engine = Engine::create().expect("Failed to create engine");
        engine
            .with_default_modules()
            .expect("Failed to register std");
        engine
            .set_message_callback(|msg| {
                println!("AngelScript: {}", msg.message);
            })
            .expect("Failed to set message callback");

        // Create or reuse a module for the test script
        let module = engine
            .get_module("TestModule", GetModuleFlags::CreateIfNotExists)
            .expect("Failed to get module");
        module
            .add_script_section_simple("test_script", script)
            .expect("Failed to add script section");
        module.build().expect("Failed to build module");

        let func = module
            .get_function_by_decl("bool not_empty()")
            .expect("Failed to get function");
        let ctx = engine.create_context().expect("Failed to create context");
        ctx.prepare(&func).expect("Failed to prepare context");
        ctx.execute().expect("Failed to execute script");

        let result = ctx.get_address_of_return_value::<bool>();
        assert!(!result.unwrap());

        let func = module
            .get_function_by_decl("bool empty()")
            .expect("Failed to get function");
        let ctx = engine.create_context().expect("Failed to create context");
        ctx.prepare(&func).expect("Failed to prepare context");
        ctx.execute().expect("Failed to execute script");

        let result = ctx.get_address_of_return_value::<bool>();
        assert!(result.unwrap());
    }

    #[test]
    fn test_string_index() {
        let script = r#"
            uint8 test() {
                string a = "Hello";
                return a[0];
            }
            uint8 oob() {
                string a = "Hello";
                return a[6];
            }
        "#;

        let mut engine = Engine::create().expect("Failed to create engine");
        engine
            .with_default_modules()
            .expect("Failed to register std");
        engine
            .set_message_callback(|msg| {
                println!("AngelScript: {}", msg.message);
            })
            .expect("Failed to set message callback");

        // Create or reuse a module for the test script
        let module = engine
            .get_module("TestModule", GetModuleFlags::CreateIfNotExists)
            .expect("Failed to get module");
        module
            .add_script_section_simple("test_script", script)
            .expect("Failed to add script section");
        module.build().expect("Failed to build module");

        let func = module
            .get_function_by_decl("uint8 test()")
            .expect("Failed to get function");
        let ctx = engine.create_context().expect("Failed to create context");
        ctx.prepare(&func).expect("Failed to prepare context");
        ctx.execute().expect("Failed to execute script");

        let result = ctx.get_return_byte();
        assert_eq!(result as char, 'H');

        let func = module
            .get_function_by_decl("uint8 oob()")
            .expect("Failed to get function");
        let ctx = engine.create_context().expect("Failed to create context");
        ctx.prepare(&func).expect("Failed to prepare context");
        let state = ctx.execute().expect("Failed to execute script");

        assert_eq!(state, ContextState::Exception);
    }

    // #[test]
    // fn test_op_add() {
    //     let script = r#"
    //         void test() {
    //             string a = "Hello";
    //             string b = " World";
    //             string c = a + b;
    //             assert(c == "Hello World");
    //         }
    //     "#;
    //
    //     let result = execute_script(script, "void test()");
    //     assert!(result.is_ok());
    // }
    //
    // #[test]
    // fn test_substr() {
    //     let script = r#"
    //         void test() {
    //             string a = "Hello, world!";
    //             string sub = a.substr(0, 5);
    //             assert(sub == "Hello");
    //             sub = a.substr(7, 5);
    //             assert(sub == "world");
    //         }
    //     "#;
    //
    //     let result = execute_script(script, "void test()");
    //     assert!(result.is_ok());
    // }
    //
    // #[test]
    // fn test_op_assign_multiple_types() {
    //     let script = r#"
    //         void test() {
    //             string a;
    //             int i = 42;
    //             a = i;
    //             assert(a == "42");
    //
    //             float f = 3.14f;
    //             a = f;
    //             assert(a == "3.14");
    //
    //             bool b = true;
    //             a = b;
    //             assert(a == "true");
    //
    //             b = false;
    //             a = b;
    //             assert(a == "false");
    //         }
    //     "#;
    //
    //     let result = execute_script(script, "void test()");
    //     assert!(result.is_ok());
    // }
    //
    // #[test]
    // fn test_op_add_multiple_types() {
    //     let script = r#"
    //         void test() {
    //             string a = "Number: ";
    //             a += 42;
    //             assert(a == "Number: 42");
    //
    //             a += 3.14f;
    //             assert(a == "Number: 423.14");
    //         }
    //     "#;
    //
    //     let result = execute_script(script, "void test()");
    //     assert!(result.is_ok());
    // }
}
