#[cfg(test)]
mod tests {
    use angelscript_core::core::context::Context;
    use angelscript_core::core::engine::Engine;
    use angelscript_core::types::enums::{ContextState, GetModuleFlags};

    // Helper function to reduce boilerplate
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

    fn execute_script_with_return<T>(
        script: &str,
        func_decl: &str,
        get_result: impl FnOnce(&Context) -> T,
    ) -> T {
        let engine = create_test_engine();

        let module = engine
            .get_module("TestModule", GetModuleFlags::CreateIfNotExists)
            .expect("Failed to get module");
        module
            .add_script_section("test_script", script, 0)
            .expect("Failed to add script section");
        module.build().expect("Failed to build module");

        let func = module
            .get_function_by_decl(func_decl)
            .expect("Failed to get function");
        let ctx = engine.create_context().expect("Failed to create context");
        ctx.prepare(&func).expect("Failed to prepare context");
        ctx.execute().expect("Failed to execute script");

        let result = get_result(&ctx);

        ctx.release().expect("Failed to release context");

        result
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
                return a.len();
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
                return a.is_empty();
            }
            bool empty() {
                string b = "";
                return b.is_empty();
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
            .add_script_section("test_script", script, 0)
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
                int i = 42;
                a = i;
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

    #[test]
    fn test_global_format_functions() {
        let script = r#"
            string test_format_int() {
                return format("%08x", 255);
            }
            string test_format_uint() {
                return format("%04X", 255);
            }
            string test_format_float() {
                return format("%0.2f", 3.14159);
            }
            string test_format_float_default() {
                return format("%f", 3.14159);
            }
        "#;

        let result = execute_script_with_return(script, "string test_format_int()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "000000ff");

        let result = execute_script_with_return(script, "string test_format_uint()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "00FF");

        let result = execute_script_with_return(script, "string test_format_float()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "3.14");

        let result =
            execute_script_with_return(script, "string test_format_float_default()", |ctx| {
                ctx.get_return_object::<String>().unwrap()
            });
        assert_eq!(result, "3.141590");
    }

    #[test]
    fn test_global_parse_functions() {
        let script = r#"
            int64 test_parse_int() {
                return parse_int("123", 10);
            }
            uint64 test_parse_uint_hex() {
                return parse_uint("FF", 16);
            }
            float test_parse_float() {
                return parse_float("3.14159");
            }
            double test_parse_double() {
                return parse_double("3.14153232283212329");
            }
            bool test_parse_bool() {
                return parse_bool("true");
            }
        "#;

        let result = execute_script_with_return(script, "int64 test_parse_int()", |ctx| {
            ctx.get_return_qword() as i64
        });
        assert_eq!(result, 123);

        let result = execute_script_with_return(script, "uint64 test_parse_uint_hex()", |ctx| {
            ctx.get_return_qword()
        });
        assert_eq!(result, 255);

        let result = execute_script_with_return(script, "float test_parse_float()", |ctx| {
            ctx.get_return_float()
        });
        assert!((result - 3.14159).abs() < 0.00001);

        let result = execute_script_with_return(script, "double test_parse_double()", |ctx| {
            ctx.get_return_double()
        });
        assert!((result - 3.14153232283212329).abs() < 0.00001);

        let result = execute_script_with_return(script, "bool test_parse_bool()", |ctx| {
            ctx.get_return_byte()
        });
        assert_eq!(result, 1u8);
    }

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

    #[test]
    fn test_string_find() {
        let script = r#"
        int test_find_exists() {
            string a = "Hello, world!";
            return a.find("world");
        }
        int test_find_not_exists() {
            string a = "Hello, world!";
            return a.find("xyz");
        }
        int test_find_empty() {
            string a = "Hello";
            return a.find("");
        }
        int test_find_at_start() {
            string a = "Hello, world!";
            return a.find("Hello");
        }
        int test_find_at_end() {
            string a = "Hello, world!";
            return a.find("!");
        }
    "#;

        let result = execute_script_with_return(script, "int test_find_exists()", |ctx| {
            ctx.get_return_dword() as i32
        });
        assert_eq!(result, 7);

        let result = execute_script_with_return(script, "int test_find_not_exists()", |ctx| {
            ctx.get_return_dword() as i32
        });
        assert_eq!(result, -1);

        let result = execute_script_with_return(script, "int test_find_empty()", |ctx| {
            ctx.get_return_dword() as i32
        });
        assert_eq!(result, 0);

        let result = execute_script_with_return(script, "int test_find_at_start()", |ctx| {
            ctx.get_return_dword() as i32
        });
        assert_eq!(result, 0);

        let result = execute_script_with_return(script, "int test_find_at_end()", |ctx| {
            ctx.get_return_dword() as i32
        });
        assert_eq!(result, 12);
    }

    #[test]
    fn test_string_rfind() {
        let script = r#"
        int test_rfind_exists() {
            string a = "Hello, world, world!";
            return a.rfind("world");
        }
        int test_rfind_not_exists() {
            string a = "Hello, world!";
            return a.rfind("xyz");
        }
        int test_rfind_single_occurrence() {
            string a = "Hello, world!";
            return a.rfind("Hello");
        }
        int test_rfind_empty() {
            string a = "Hello";
            return a.rfind("");
        }
    "#;

        let result = execute_script_with_return(script, "int test_rfind_exists()", |ctx| {
            ctx.get_return_dword() as i32
        });
        assert_eq!(result, 14); // Last occurrence of "world"

        let result = execute_script_with_return(script, "int test_rfind_not_exists()", |ctx| {
            ctx.get_return_dword() as i32
        });
        assert_eq!(result, -1);

        let result =
            execute_script_with_return(script, "int test_rfind_single_occurrence()", |ctx| {
                ctx.get_return_dword() as i32
            });
        assert_eq!(result, 0);

        let result = execute_script_with_return(script, "int test_rfind_empty()", |ctx| {
            ctx.get_return_dword() as i32
        });
        assert_eq!(result, 5); // Should return length of string for empty pattern
    }

    #[test]
    fn test_string_insert_str() {
        let script = r#"
        string test_insert_beginning() {
            string a = "world!";
            a.insert_str(0, "Hello, ");
            return a;
        }
        string test_insert_middle() {
            string a = "Hello!";
            a.insert_str(5, ", world");
            return a;
        }
        string test_insert_end() {
            string a = "Hello";
            a.insert_str(5, ", world!");
            return a;
        }
        string test_insert_empty() {
            string a = "Hello, world!";
            a.insert_str(7, "");
            return a;
        }
    "#;

        let result = execute_script_with_return(script, "string test_insert_beginning()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "Hello, world!");

        let result = execute_script_with_return(script, "string test_insert_middle()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "Hello, world!");

        let result = execute_script_with_return(script, "string test_insert_end()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "Hello, world!");

        let result = execute_script_with_return(script, "string test_insert_empty()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_string_push_str() {
        let script = r#"
        string test_push_str_basic() {
            string a = "Hello";
            a.push_str(", world!");
            return a;
        }
        string test_push_str_empty_target() {
            string a = "";
            a.push_str("Hello, world!");
            return a;
        }
        string test_push_str_empty_source() {
            string a = "Hello, world!";
            a.push_str("");
            return a;
        }
        string test_push_str_multiple() {
            string a = "Hello";
            a.push_str(", ");
            a.push_str("world");
            a.push_str("!");
            return a;
        }
    "#;

        let result = execute_script_with_return(script, "string test_push_str_basic()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "Hello, world!");

        let result =
            execute_script_with_return(script, "string test_push_str_empty_target()", |ctx| {
                ctx.get_return_object::<String>().unwrap()
            });
        assert_eq!(result, "Hello, world!");

        let result =
            execute_script_with_return(script, "string test_push_str_empty_source()", |ctx| {
                ctx.get_return_object::<String>().unwrap()
            });
        assert_eq!(result, "Hello, world!");

        let result = execute_script_with_return(script, "string test_push_str_multiple()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_string_clear() {
        let script = r#"
        bool test_clear_non_empty() {
            string a = "Hello, world!";
            a.clear();
            return a.is_empty();
        }
        bool test_clear_empty() {
            string a = "";
            a.clear();
            return a.is_empty();
        }
        uint test_clear_length() {
            string a = "Hello, world!";
            a.clear();
            return a.len();
        }
        string test_clear_and_assign() {
            string a = "Hello, world!";
            a.clear();
            a = "New content";
            return a;
        }
    "#;

        let result = execute_script_with_return(script, "bool test_clear_non_empty()", |ctx| {
            ctx.get_address_of_return_value::<bool>().unwrap()
        });
        assert!(result);

        let result = execute_script_with_return(script, "bool test_clear_empty()", |ctx| {
            ctx.get_address_of_return_value::<bool>().unwrap()
        });
        assert!(result);

        let result = execute_script_with_return(script, "uint test_clear_length()", |ctx| {
            ctx.get_return_dword()
        });
        assert_eq!(result, 0);

        let result = execute_script_with_return(script, "string test_clear_and_assign()", |ctx| {
            ctx.get_return_object::<String>().unwrap()
        });
        assert_eq!(result, "New content");
    }
}
