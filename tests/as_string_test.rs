#[cfg(test)]
mod tests {
    use angelscript::{AngelScript, GetModuleFlags};

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

        let mut engine = AngelScript::create_script_engine().expect("Failed to create engine");
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

        println!("Got result: {:?}", ctx.get_return_object::<String>());

        let result = ctx.get_return_object::<String>();
        assert_eq!(result.unwrap().read(), "World");

        println!("End of test");
    }

    // #[test]
    // fn test_op_add_assign() {
    //     let script = r#"
    //         void test() {
    //             string a = "Hello";
    //             a += " World";
    //             assert(a == "Hello World");
    //         }
    //     "#;
    //
    //     let result = execute_script(script, "void test()");
    //     assert!(result.is_ok());
    // }
    //
    // #[test]
    // fn test_op_equals() {
    //     let script = r#"
    //         void test() {
    //             string a = "test";
    //             string b = "test";
    //             string c = "different";
    //             assert(a == b);
    //             assert(!(a == c));
    //         }
    //     "#;
    //
    //     let result = execute_script(script, "void test()");
    //     assert!(result.is_ok());
    // }
    //
    // #[test]
    // fn test_op_cmp() {
    //     let script = r#"
    //         void test() {
    //             string a = "abc";
    //             string b = "bcd";
    //             string c = "abc";
    //             assert(a < b);
    //             assert(!(b < a));
    //             assert(a == c);
    //         }
    //     "#;
    //
    //     let result = execute_script(script, "void test()");
    //     assert!(result.is_ok());
    // }
    //
    // #[test]
    // fn test_string_length() {
    //     let script = r#"
    //         void test() {
    //             string a = "Hello";
    //             assert(a.length() == 5);
    //             string b = "";
    //             assert(b.length() == 0);
    //         }
    //     "#;
    //
    //     let result = execute_script(script, "void test()");
    //     assert!(result.is_ok());
    // }
    //
    // #[test]
    // fn test_string_is_empty() {
    //     let script = r#"
    //         void test() {
    //             string a = "Hello";
    //             assert(!a.isEmpty());
    //             string b = "";
    //             assert(b.isEmpty());
    //         }
    //     "#;
    //
    //     let result = execute_script(script, "void test()");
    //     assert!(result.is_ok());
    // }
    //
    // #[test]
    // fn test_string_index() {
    //     let script = r#"
    //         void test() {
    //             string a = "Hello";
    //             assert(a[0] == 'H');
    //             assert(a[4] == 'o');
    //             // Ensure out-of-bounds access is handled (requires specific code setup)
    //         }
    //     "#;
    //
    //     let result = execute_script(script, "void test()");
    //     assert!(result.is_ok());
    // }
    //
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
