#[cfg(test)]
mod tests {
    use angelscript_core::core::context::Context;
    use angelscript_core::core::engine::Engine;
    use angelscript_core::types::enums::GetModuleFlags;

    // Helper function to reduce boilerplate
    fn create_test_engine() -> Engine {
        let mut engine = Engine::create().expect("Failed to create engine");
        engine
            .install(angelscript_addons::math::addon()) // Your math addon
            .expect("Failed to install math addon");
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

    // Helper function for approximate equality
    fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
        (a - b).abs() < epsilon
    }

    #[test]
    fn test_basic_trigonometry() {
        let script = r#"
            double test_cos() {
                return cos(0.0);
            }
            double test_sin() {
                return sin(0.0);
            }
            double test_tan() {
                return tan(0.0);
            }
        "#;

        let result = execute_script_with_return(script, "double test_cos()", |ctx| {
            ctx.get_return_double()
        });
        println!("cos(0.0) = {}", result);
        assert!(approx_eq(result, 1.0, 1e-10));

        let result = execute_script_with_return(script, "double test_sin()", |ctx| {
            ctx.get_return_double()
        });
        println!("sin(0.0) = {}", result);
        assert!(approx_eq(result, 0.0, 1e-10));

        let result = execute_script_with_return(script, "double test_tan()", |ctx| {
            ctx.get_return_double()
        });
        println!("tan(0.0) = {}", result);
        assert!(approx_eq(result, 0.0, 1e-10));
    }

    #[test]
    fn test_simple_calculations() {
        let script = r#"
            double test_sqrt() {
                return sqrt(4.0);
            }
            double test_abs_positive() {
                return abs(5.0);
            }
            double test_abs_negative() {
                return abs(-5.0);
            }
            double test_floor() {
                return floor(3.7);
            }
            double test_ceil() {
                return ceil(3.2);
            }
        "#;

        let result = execute_script_with_return(script, "double test_sqrt()", |ctx| {
            ctx.get_return_double()
        });
        println!("sqrt(4.0) = {}", result);
        assert!(approx_eq(result, 2.0, 1e-10));

        let result = execute_script_with_return(script, "double test_abs_positive()", |ctx| {
            ctx.get_return_double()
        });
        println!("abs(5.0) = {}", result);
        assert!(approx_eq(result, 5.0, 1e-10));

        let result = execute_script_with_return(script, "double test_abs_negative()", |ctx| {
            ctx.get_return_double()
        });
        println!("abs(-5.0) = {}", result);
        assert!(approx_eq(result, 5.0, 1e-10));

        let result = execute_script_with_return(script, "double test_floor()", |ctx| {
            ctx.get_return_double()
        });
        println!("floor(3.7) = {}", result);
        assert!(approx_eq(result, 3.0, 1e-10));

        let result = execute_script_with_return(script, "double test_ceil()", |ctx| {
            ctx.get_return_double()
        });
        println!("ceil(3.2) = {}", result);
        assert!(approx_eq(result, 4.0, 1e-10));
    }

    #[test]
    fn test_two_argument_functions() {
        let script = r#"
            double test_powf() {
                return powf(2.0, 3.0);
            }
            double test_max() {
                return max(5.0, 3.0);
            }
            double test_min() {
                return min(5.0, 3.0);
            }
            double test_hypot() {
                return hypot(3.0, 4.0);
            }
        "#;

        let result = execute_script_with_return(script, "double test_powf()", |ctx| {
            ctx.get_return_double()
        });
        println!("powf(2.0, 3.0) = {}", result);
        assert!(approx_eq(result, 8.0, 1e-10));

        let result = execute_script_with_return(script, "double test_max()", |ctx| {
            ctx.get_return_double()
        });
        println!("max(5.0, 3.0) = {}", result);
        assert!(approx_eq(result, 5.0, 1e-10));

        let result = execute_script_with_return(script, "double test_min()", |ctx| {
            ctx.get_return_double()
        });
        println!("min(5.0, 3.0) = {}", result);
        assert!(approx_eq(result, 3.0, 1e-10));

        let result = execute_script_with_return(script, "double test_hypot()", |ctx| {
            ctx.get_return_double()
        });
        println!("hypot(3.0, 4.0) = {}", result);
        assert!(approx_eq(result, 5.0, 1e-10));
    }

    #[test]
    fn test_logarithms() {
        let script = r#"
            double test_ln() {
                return ln(2.718281828459045);
            }
            double test_log10() {
                return log10(10.0);
            }
            double test_log2() {
                return log2(8.0);
            }
        "#;

        let result = execute_script_with_return(script, "double test_ln()", |ctx| {
            ctx.get_return_double()
        });
        println!("ln(e) = {}", result);
        assert!(approx_eq(result, 1.0, 1e-10));

        let result = execute_script_with_return(script, "double test_log10()", |ctx| {
            ctx.get_return_double()
        });
        println!("log10(10.0) = {}", result);
        assert!(approx_eq(result, 1.0, 1e-10));

        let result = execute_script_with_return(script, "double test_log2()", |ctx| {
            ctx.get_return_double()
        });
        println!("log2(8.0) = {}", result);
        assert!(approx_eq(result, 3.0, 1e-10));
    }

    #[test]
    fn test_exponentials() {
        let script = r#"
            double test_exp() {
                return exp(1.0);
            }
            double test_exp2() {
                return exp2(3.0);
            }
        "#;

        let result = execute_script_with_return(script, "double test_exp()", |ctx| {
            ctx.get_return_double()
        });
        println!("exp(1.0) = {}", result);
        println!("Expected E = {}", std::f64::consts::E);
        assert!(approx_eq(result, std::f64::consts::E, 1e-10));

        let result = execute_script_with_return(script, "double test_exp2()", |ctx| {
            ctx.get_return_double()
        });
        println!("exp2(3.0) = {}", result);
        assert!(approx_eq(result, 8.0, 1e-10));
    }

    #[test]
    fn test_trigonometry_with_known_values() {
        let script = r#"
            double test_sin_30() {
                return sin(0.5235987755982988); // 30 degrees in radians
            }
            double test_cos_60() {
                return cos(1.0471975511965976); // 60 degrees in radians
            }
            double test_tan_45() {
                return tan(0.7853981633974483); // 45 degrees in radians
            }
        "#;

        let result = execute_script_with_return(script, "double test_sin_30()", |ctx| {
            ctx.get_return_double()
        });
        println!("sin(30°) = {}", result);
        assert!(approx_eq(result, 0.5, 1e-10));

        let result = execute_script_with_return(script, "double test_cos_60()", |ctx| {
            ctx.get_return_double()
        });
        println!("cos(60°) = {}", result);
        assert!(approx_eq(result, 0.5, 1e-10));

        let result = execute_script_with_return(script, "double test_tan_45()", |ctx| {
            ctx.get_return_double()
        });
        println!("tan(45°) = {}", result);
        assert!(approx_eq(result, 1.0, 1e-10));
    }

    #[test]
    fn test_rounding() {
        let script = r#"
            double test_round_up() {
                return round(3.6);
            }
            double test_round_down() {
                return round(3.4);
            }
            double test_trunc_positive() {
                return trunc(3.8);
            }
            double test_trunc_negative() {
                return trunc(-3.8);
            }
        "#;

        let result = execute_script_with_return(script, "double test_round_up()", |ctx| {
            ctx.get_return_double()
        });
        println!("round(3.6) = {}", result);
        assert!(approx_eq(result, 4.0, 1e-10));

        let result = execute_script_with_return(script, "double test_round_down()", |ctx| {
            ctx.get_return_double()
        });
        println!("round(3.4) = {}", result);
        assert!(approx_eq(result, 3.0, 1e-10));

        let result = execute_script_with_return(script, "double test_trunc_positive()", |ctx| {
            ctx.get_return_double()
        });
        println!("trunc(3.8) = {}", result);
        assert!(approx_eq(result, 3.0, 1e-10));

        let result = execute_script_with_return(script, "double test_trunc_negative()", |ctx| {
            ctx.get_return_double()
        });
        println!("trunc(-3.8) = {}", result);
        assert!(approx_eq(result, -3.0, 1e-10));
    }

    #[test]
    fn test_sign_functions() {
        let script = r#"
            double test_signum_positive() {
                return signum(5.5);
            }
            double test_signum_negative() {
                return signum(-5.5);
            }
            double test_signum_zero() {
                return signum(0.0);
            }
        "#;

        let result = execute_script_with_return(script, "double test_signum_positive()", |ctx| {
            ctx.get_return_double()
        });
        println!("signum(5.5) = {}", result);
        assert!(approx_eq(result, 1.0, 1e-10));

        let result = execute_script_with_return(script, "double test_signum_negative()", |ctx| {
            ctx.get_return_double()
        });
        println!("signum(-5.5) = {}", result);
        assert!(approx_eq(result, -1.0, 1e-10));

        let result = execute_script_with_return(script, "double test_signum_zero()", |ctx| {
            ctx.get_return_double()
        });
        println!("signum(0.0) = {}", result);
        // Note: signum(0.0) might return 0.0 or 1.0 depending on implementation
        assert!(result == 0.0 || result == 1.0);
    }

    #[test]
    fn test_angle_conversion() {
        let script = r#"
            double test_to_degrees() {
                return to_degrees(1.5707963267948966); // PI/2 radians
            }
            double test_to_radians() {
                return to_radians(90.0); // 90 degrees
            }
        "#;

        let result = execute_script_with_return(script, "double test_to_degrees()", |ctx| {
            ctx.get_return_double()
        });
        println!("to_degrees(PI/2) = {}", result);
        assert!(approx_eq(result, 90.0, 1e-10));

        let result = execute_script_with_return(script, "double test_to_radians()", |ctx| {
            ctx.get_return_double()
        });
        println!("to_radians(90.0) = {}", result);
        assert!(approx_eq(result, std::f64::consts::PI / 2.0, 1e-10));
    }
}
