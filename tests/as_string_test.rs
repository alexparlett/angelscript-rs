#[cfg(test)]
mod tests {
    use angelscript::core::engine::Engine;
    use angelscript::prelude::{ContextState, GetModuleFlags};
    use angelscript_core::core::script_generic::ScriptGeneric;

    // Helper function to reduce boilerplate
    fn create_test_engine() -> Engine {
        let mut engine = Engine::create().expect("Failed to create engine");
        engine
            .install(angelscript::addons::string::addon())
            .expect("Failed to install string addon");
        engine
            .set_message_callback(|msg| {
                println!("AngelScript: {}", msg.message);
            })
            .expect("Failed to set message callback");
        engine
    }

    fn execute_script_with_return<T>(
        script: &str,
        func_decl: &str,
        get_result: impl FnOnce(&angelscript::core::context::Context) -> T,
    ) -> T {
        let engine = create_test_engine();

        engine
            .register_global_function(
                "void print(const string &in)",
                |g: &ScriptGeneric| {
                    let arg_ptr = g.get_arg_object(0).unwrap();
                    println!("Hello, {}", arg_ptr.as_ref::<String>());
                },
                None,
            )
            .expect("Failed to register print function");

        let module = engine
            .get_module("TestModule", GetModuleFlags::CreateIfNotExists)
            .expect("Failed to get module");
        module
            .add_script_section_simple("test_script", script)
            .expect("Failed to add script section");
        module.build().expect("Failed to build module");

        let func = module
            .get_function_by_decl(func_decl)
            .expect("Failed to get function");
        let ctx = engine.create_context().expect("Failed to create context");
        ctx.prepare(&func).expect("Failed to prepare context");
        ctx.execute().expect("Failed to execute script");

        get_result(&ctx)
    }

    #[test]
    fn test_string_literal() {
        let script = r#"
            string test() {
                return "James";
            }
        "#;

        let result = execute_script_with_return(script, "string test()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "James");
    }

    #[test]
    fn test_string_constructor() {
        let script = r#"
            string test() {
                string a("Fred");
                return a;
            }
        "#;

        let result = execute_script_with_return(script, "string test()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "Fred");
    }

    #[test]
    fn test_op_assign() {
        let script = r#"
            string test() {
                string a;
                a = "World";
                return a;
            }
        "#;

        let result = execute_script_with_return(script, "string test()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "World");
    }

    #[test]
    fn test_op_add_assign() {
        let script = r#"
            string test() {
                string a("Hello");
                a += " World";
                return a;
            }
        "#;

        let result = execute_script_with_return(script, "string test()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_op_equals() {
        let script = r#"
            bool equal() {
                string a("test");
                string b("test");
                return a == b;
            }

            bool not_equal() {
                string a("test");
                string b("different");
                return a == b;
            }
        "#;

        let result = execute_script_with_return(script, "bool equal()", |ctx| {
            ctx.get_address_of_return_value::<bool>().unwrap()
        });
        assert!(result);

        let result = execute_script_with_return(script, "bool not_equal()", |ctx| {
            ctx.get_address_of_return_value::<bool>().unwrap()
        });
        assert!(!result);
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

        let result = execute_script_with_return(script, "bool lt()", |ctx| {
            ctx.get_address_of_return_value::<bool>().unwrap()
        });
        assert!(result);

        let result = execute_script_with_return(script, "bool gt()", |ctx| {
            ctx.get_address_of_return_value::<bool>().unwrap()
        });
        assert!(result);
    }

    #[test]
    fn test_string_length() {
        let script = r#"
            uint test() {
                string a = "Hello";
                return a.length();
            }
        "#;

        let result =
            execute_script_with_return(script, "uint test()", |ctx| ctx.get_return_dword());
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

        let result = execute_script_with_return(script, "bool not_empty()", |ctx| {
            ctx.get_address_of_return_value::<bool>().unwrap()
        });
        assert!(!result);

        let result = execute_script_with_return(script, "bool empty()", |ctx| {
            ctx.get_address_of_return_value::<bool>().unwrap()
        });
        assert!(result);
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

        let result =
            execute_script_with_return(script, "uint8 test()", |ctx| ctx.get_return_byte());
        assert_eq!(result as char, 'H');

        // Test out of bounds - should throw exception
        let engine = create_test_engine();
        let module = engine
            .get_module("TestModule", GetModuleFlags::CreateIfNotExists)
            .expect("Failed to get module");
        module
            .add_script_section_simple("test_script", script)
            .expect("Failed to add script section");
        module.build().expect("Failed to build module");

        let func = module
            .get_function_by_decl("uint8 oob()")
            .expect("Failed to get function");
        let ctx = engine.create_context().expect("Failed to create context");
        ctx.prepare(&func).expect("Failed to prepare context");
        let state = ctx.execute().expect("Failed to execute script");

        assert_eq!(state, ContextState::Exception);
    }

    // New tests for the missing functionality
    #[test]
    fn test_op_add() {
        let script = r#"
            string test() {
                string a = "Hello";
                string b = " World";
                string c = a + b;
                return c;
            }
        "#;

        let result = execute_script_with_return(script, "string test()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_substr() {
        let script = r#"
            string test_basic() {
                string a = "Hello, world!";
                return a.substr(0, 5);
            }
            string test_middle() {
                string a = "Hello, world!";
                return a.substr(7, 5);
            }
            string test_to_end() {
                string a = "Hello, world!";
                return a.substr(7, -1);
            }
        "#;

        let result = execute_script_with_return(script, "string test_basic()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "Hello");

        let result = execute_script_with_return(script, "string test_middle()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "world");

        let result = execute_script_with_return(script, "string test_to_end()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "world!");
    }

    #[test]
    fn test_op_assign_multiple_types() {
        let script = r#"
            string test_int() {
                string a;
                print("created string");
                int i = 42;
                print("created int");
                a = i;
                print("assigned int to string");
                print(a);
                return a;
            }
            string test_float() {
                string a;
                float f = 3.14f;
                a = f;
                return a;
            }
            string test_bool_true() {
                string a;
                bool b = true;
                a = b;
                return a;
            }
            string test_bool_false() {
                string a;
                bool b = false;
                a = b;
                return a;
            }
        "#;

        let result = execute_script_with_return(script, "string test_int()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "42");

        let result = execute_script_with_return(script, "string test_float()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "3.14");

        let result = execute_script_with_return(script, "string test_bool_true()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "true");

        let result = execute_script_with_return(script, "string test_bool_false()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "false");
    }

    #[test]
    fn test_op_add_assign_multiple_types() {
        let script = r#"
            string test() {
                string a = "Number: ";
                a += 42;
                a += ", Float: ";
                a += 3.14f;
                a += ", Bool: ";
                a += true;
                return a;
            }
        "#;

        let result = execute_script_with_return(script, "string test()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "Number: 42, Float: 3.14, Bool: true");
    }

    // #[test]
    // fn test_string_find_methods() {
    //     let script = r#"
    //         int test_find_first() {
    //             string text = "Hello World Hello";
    //             return text.findFirst("Hello", 0);
    //         }
    //         int test_find_first_offset() {
    //             string text = "Hello World Hello";
    //             return text.findFirst("Hello", 1);
    //         }
    //         int test_find_first_not_found() {
    //             string text = "Hello World";
    //             return text.findFirst("xyz", 0);
    //         }
    //         int test_find_first_of() {
    //             string text = "Hello World";
    //             return text.findFirstOf("aeiou", 0);
    //         }
    //         int test_find_first_not_of() {
    //             string text = "Hello World";
    //             return text.findFirstNotOf("Helo ", 0);
    //         }
    //     "#;
    //
    //     let result = execute_script_with_return(script, "int test_find_first()", |ctx| {
    //         ctx.get_return_dword() as i32
    //     });
    //     assert_eq!(result, 0);
    //
    //     let result = execute_script_with_return(script, "int test_find_first_offset()", |ctx| {
    //         ctx.get_return_dword() as i32
    //     });
    //     assert_eq!(result, 12);
    //
    //     let result = execute_script_with_return(script, "int test_find_first_not_found()", |ctx| {
    //         ctx.get_return_dword() as i32
    //     });
    //     assert_eq!(result, -1);
    //
    //     let result = execute_script_with_return(script, "int test_find_first_of()", |ctx| {
    //         ctx.get_return_dword() as i32
    //     });
    //     assert_eq!(result, 1); // 'e' in "Hello"
    //
    //     let result = execute_script_with_return(script, "int test_find_first_not_of()", |ctx| {
    //         ctx.get_return_dword() as i32
    //     });
    //     assert_eq!(result, 6); // 'W' in "World"
    // }
    //
    // #[test]
    // fn test_string_manipulation() {
    //     let script = r#"
    //         string test_insert() {
    //             string text = "Hello World";
    //             text.insert(6, "Beautiful ");
    //             return text;
    //         }
    //         string test_erase() {
    //             string text = "Hello Beautiful World";
    //             text.erase(6, 10); // Remove "Beautiful "
    //             return text;
    //         }
    //         string test_erase_to_end() {
    //             string text = "Hello World";
    //             text.erase(5, -1); // Remove from position 5 to end
    //             return text;
    //         }
    //     "#;
    //
    //     let result = execute_script_with_return(script, "string test_insert()", |ctx| {
    //         ctx.get_return_object::<String>().unwrap()
    //     });
    //     assert_eq!(result, "Hello Beautiful World");
    //
    //     let result = execute_script_with_return(script, "string test_erase()", |ctx| {
    //         ctx.get_return_object::<String>().unwrap()
    //     });
    //     assert_eq!(result, "Hello World");
    //
    //     let result = execute_script_with_return(script, "string test_erase_to_end()", |ctx| {
    //         ctx.get_return_object::<String>().unwrap()
    //     });
    //     assert_eq!(result, "Hello");
    // }
    //
    // #[test]
    // fn test_global_format_functions() {
    //     let script = r#"
    //         string test_format_int() {
    //             return formatInt(255, "x", 8);
    //         }
    //         string test_format_uint() {
    //             return formatUInt(255, "X", 4);
    //         }
    //         string test_format_float() {
    //             return formatFloat(3.14159, "f", 0, 2);
    //         }
    //         string test_format_float_default() {
    //             return formatFloat(3.14159, "", 0, 0);
    //         }
    //     "#;
    //
    //     let result = execute_script_with_return(script, "string test_format_int()", |ctx| {
    //         ctx.get_return_object::<String>().unwrap()
    //     });
    //     assert_eq!(result, "000000ff");
    //
    //     let result = execute_script_with_return(script, "string test_format_uint()", |ctx| {
    //         ctx.get_return_object::<String>().unwrap()
    //     });
    //     assert_eq!(result, "00FF");
    //
    //     let result = execute_script_with_return(script, "string test_format_float()", |ctx| {
    //         ctx.get_return_object::<String>().unwrap()
    //     });
    //     assert_eq!(result, "3.14");
    //
    //     let result = execute_script_with_return(script, "string test_format_float_default()", |ctx| {
    //         ctx.get_return_object::<String>().unwrap()
    //     });
    //     assert_eq!(result, "3.14159"); // ryu formatting
    // }
    //
    // #[test]
    // fn test_global_parse_functions() {
    //     let script = r#"
    //         int64 test_parse_int() {
    //             uint byteCount;
    //             return parseInt("123abc", 10, byteCount);
    //         }
    //         uint64 test_parse_uint_hex() {
    //             uint byteCount;
    //             return parseUInt("FF", 16, byteCount);
    //         }
    //         double test_parse_float() {
    //             uint byteCount;
    //             return parseFloat("3.14159", byteCount);
    //         }
    //     "#;
    //
    //     let result = execute_script_with_return(script, "int64 test_parse_int()", |ctx| {
    //         ctx.get_return_qword() as i64
    //     });
    //     assert_eq!(result, 123);
    //
    //     let result = execute_script_with_return(script, "uint64 test_parse_uint_hex()", |ctx| {
    //         ctx.get_return_qword()
    //     });
    //     assert_eq!(result, 255);
    //
    //     let result = execute_script_with_return(script, "double test_parse_float()", |ctx| {
    //         ctx.get_return_double()
    //     });
    //     assert!((result - 3.14159).abs() < 0.00001);
    // }
    
    #[test]
    fn test_reverse_operations() {
        let script = r#"
            string test_int_add_string() {
                return 42 + " is the answer";
            }
            string test_float_add_string() {
                return 3.14f + " is pi";
            }
            string test_bool_add_string() {
                return true + " story";
            }
        "#;
    
        let result = execute_script_with_return(script, "string test_int_add_string()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "42 is the answer");
    
        let result = execute_script_with_return(script, "string test_float_add_string()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "3.14 is pi");
    
        let result = execute_script_with_return(script, "string test_bool_add_string()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "true story");
    }
    
    #[test]
    fn test_complex_string_operations() {
        let script = r#"
            string test() {
                string result = "Numbers: ";
                result += 1;
                result += ", ";
                result += 2.5f;
                result += ", ";
                result += true;
    
                string sub = result.substr(9, 1); // Extract "1"
                result += " | First: " + sub;
    
                return result;
            }
        "#;
    
        let result = execute_script_with_return(script, "string test()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "Numbers: 1, 2.5, true | First: 1");
    }
}
