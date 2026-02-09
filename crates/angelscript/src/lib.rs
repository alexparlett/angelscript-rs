pub use angelscript_core as core;
pub use angelscript_macros as macros;
pub use angelscript_vm as vm;

// Re-export proc macros at the top level for ergonomic use.
pub use angelscript_macros::{script_function, ScriptType};

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{FunctionId, QualifiedName};
    use angelscript_vm::module::{Module, NativeCallContext};

    // ── ScriptType derive tests ──

    #[derive(ScriptType)]
    struct SimpleStruct {
        #[script(property)]
        health: f32,
        #[script(property)]
        level: i32,
        internal: u64, // not exposed
    }

    #[test]
    fn script_type_default_name() {
        assert_eq!(SimpleStruct::SCRIPT_NAME, "SimpleStruct");
    }

    #[test]
    fn script_type_properties() {
        let props = SimpleStruct::script_properties();
        assert_eq!(props.len(), 2);
        assert_eq!(props[0], ("health", "float"));
        assert_eq!(props[1], ("level", "int"));
    }

    #[test]
    fn script_type_property_count() {
        assert_eq!(SimpleStruct::script_property_count(), 2);
    }

    #[derive(ScriptType)]
    #[script(name = "Player")]
    struct PlayerStruct {
        #[script(property)]
        name: String,
        #[script(property)]
        score: f64,
    }

    #[test]
    fn script_type_custom_name() {
        assert_eq!(PlayerStruct::SCRIPT_NAME, "Player");
    }

    #[test]
    fn script_type_custom_name_properties() {
        let props = PlayerStruct::script_properties();
        assert_eq!(props.len(), 2);
        assert_eq!(props[0], ("name", "string"));
        assert_eq!(props[1], ("score", "double"));
    }

    #[derive(ScriptType)]
    struct NoProperties {
        hidden: i32,
    }

    #[test]
    fn script_type_no_properties() {
        assert_eq!(NoProperties::script_property_count(), 0);
        assert!(NoProperties::script_properties().is_empty());
    }

    // ── script_function tests ──

    #[script_function]
    fn add_ints(a: i32, b: i32) -> i32 {
        a + b
    }

    #[test]
    fn script_function_i32() {
        // The macro generates register_add_ints().
        let native = register_add_ints(FunctionId(100));
        assert_eq!(native.id, FunctionId(100));
        assert_eq!(native.name, QualifiedName::global("add_ints"));

        // Call the wrapper.
        let mut ctx = NativeCallContext::new(vec![7, 3]);
        (native.callback)(&mut ctx).unwrap();
        assert_eq!(ctx.return_value, 10);
        assert!(!ctx.return_is_64bit);
    }

    #[script_function]
    fn negate(x: i32) -> i32 {
        -x
    }

    #[test]
    fn script_function_single_param() {
        let native = register_negate(FunctionId(101));
        let mut ctx = NativeCallContext::new(vec![42u32]);
        (native.callback)(&mut ctx).unwrap();
        assert_eq!(ctx.return_value as i32, -42);
    }

    #[script_function]
    fn add_floats(a: f32, b: f32) -> f32 {
        a + b
    }

    #[test]
    fn script_function_f32() {
        let native = register_add_floats(FunctionId(102));
        let mut ctx = NativeCallContext::new(vec![1.5f32.to_bits(), 2.5f32.to_bits()]);
        (native.callback)(&mut ctx).unwrap();
        let result = f32::from_bits(ctx.return_value as u32);
        assert!((result - 4.0).abs() < f32::EPSILON);
    }

    #[script_function]
    fn returns_bool(x: i32) -> bool {
        x > 0
    }

    #[test]
    fn script_function_bool_return() {
        let native = register_returns_bool(FunctionId(103));

        let mut ctx = NativeCallContext::new(vec![5]);
        (native.callback)(&mut ctx).unwrap();
        assert_eq!(ctx.return_value, 1);

        let mut ctx = NativeCallContext::new(vec![(-3i32) as u32]);
        (native.callback)(&mut ctx).unwrap();
        assert_eq!(ctx.return_value, 0);
    }

    #[script_function]
    fn void_function(x: i32) {
        let _ = x; // side-effect only
    }

    #[test]
    fn script_function_void_return() {
        let native = register_void_function(FunctionId(104));
        let mut ctx = NativeCallContext::new(vec![42]);
        (native.callback)(&mut ctx).unwrap();
        // Void return — return_value unchanged (0).
        assert_eq!(ctx.return_value, 0);
    }

    #[script_function]
    fn add_doubles(a: f64, b: f64) -> f64 {
        a + b
    }

    #[test]
    fn script_function_f64() {
        let native = register_add_doubles(FunctionId(105));
        let a_bits = 1.5f64.to_bits();
        let b_bits = 2.5f64.to_bits();
        let mut ctx = NativeCallContext::new(vec![
            a_bits as u32,
            (a_bits >> 32) as u32,
            b_bits as u32,
            (b_bits >> 32) as u32,
        ]);
        (native.callback)(&mut ctx).unwrap();
        let result = f64::from_bits(ctx.return_value);
        assert!((result - 4.0).abs() < f64::EPSILON);
        assert!(ctx.return_is_64bit);
    }

    #[script_function]
    fn add_i64(a: i64, b: i64) -> i64 {
        a + b
    }

    #[test]
    fn script_function_i64() {
        let native = register_add_i64(FunctionId(106));
        let a: i64 = 100_000_000_000;
        let b: i64 = 200_000_000_000;
        let a_u = a as u64;
        let b_u = b as u64;
        let mut ctx = NativeCallContext::new(vec![
            a_u as u32,
            (a_u >> 32) as u32,
            b_u as u32,
            (b_u >> 32) as u32,
        ]);
        (native.callback)(&mut ctx).unwrap();
        assert_eq!(ctx.return_value as i64, 300_000_000_000i64);
        assert!(ctx.return_is_64bit);
    }

    // ── Integration: register into Module ──

    #[script_function]
    fn multiply(a: i32, b: i32) -> i32 {
        a * b
    }

    #[test]
    fn script_function_register_in_module() {
        let mut module = Module::new();
        let native = register_multiply(FunctionId(200));
        module.add_native_function(native);

        let found = module.find_function(&QualifiedName::global("multiply"));
        assert_eq!(found, Some(FunctionId(200)));

        let func = module.get_native_function(FunctionId(200)).unwrap();
        let mut ctx = NativeCallContext::new(vec![6, 7]);
        (func.callback)(&mut ctx).unwrap();
        assert_eq!(ctx.return_value, 42);
    }
}
