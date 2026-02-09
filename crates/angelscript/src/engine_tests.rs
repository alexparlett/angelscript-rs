use crate::*;

#[test]
fn debug_bytecode_dump() {
    let engine = Engine::new();
    let module = engine
        .compile("test", "int add(int a, int b) { return a + b; }")
        .unwrap();

    // Dump bytecode for the function.
    let name = QualifiedName::global("add");
    let fid = module.module.find_function(&name).unwrap();
    let func = module.module.get_function(fid).unwrap();
    eprintln!("bytecode len: {}", func.bytecode.data.len());
    for (i, dw) in func.bytecode.data.iter().enumerate() {
        eprintln!("  [{}]: 0x{:08X} ({})", i, dw, dw);
    }
    eprintln!("stack_size: {}", func.stack_size);
    eprintln!("param_count: {}", func.param_count);
}

#[test]
fn engine_compile_and_call_return_constant() {
    let engine = Engine::new();
    let module = engine
        .compile(
            "test",
            r#"
        int answer() { return 42; }
    "#,
        )
        .unwrap();
    let result = engine.call(&module, "answer", &[]).unwrap();
    assert_eq!(result, Value::I32(42));
}

#[test]
fn engine_compile_and_call_add() {
    let engine = Engine::new();
    let module = engine
        .compile(
            "test",
            r#"
        int add(int a, int b) { return a + b; }
    "#,
        )
        .unwrap();
    let result = engine
        .call(&module, "add", &[Value::I32(2), Value::I32(3)])
        .unwrap();
    assert_eq!(result, Value::I32(5));
}

#[test]
fn engine_compile_and_call_arithmetic() {
    let engine = Engine::new();
    let module = engine
        .compile(
            "test",
            r#"
        int calc(int x, int y) {
            int sum = x + y;
            int product = x * y;
            return sum + product;
        }
    "#,
        )
        .unwrap();
    let result = engine
        .call(&module, "calc", &[Value::I32(3), Value::I32(4)])
        .unwrap();
    // sum = 7, product = 12, result = 19
    assert_eq!(result, Value::I32(19));
}

#[test]
fn engine_compile_and_call_conditional() {
    let engine = Engine::new();
    let module = engine
        .compile(
            "test",
            r#"
        int max(int a, int b) {
            if (a > b) {
                return a;
            }
            return b;
        }
    "#,
        )
        .unwrap();

    let result = engine
        .call(&module, "max", &[Value::I32(10), Value::I32(20)])
        .unwrap();
    assert_eq!(result, Value::I32(20));

    let result = engine
        .call(&module, "max", &[Value::I32(30), Value::I32(5)])
        .unwrap();
    assert_eq!(result, Value::I32(30));
}

#[test]
fn engine_compile_and_call_loop() {
    let engine = Engine::new();
    let module = engine
        .compile(
            "test",
            r#"
        int sum_to(int n) {
            int total = 0;
            for (int i = 1; i <= n; i++) {
                total = total + i;
            }
            return total;
        }
    "#,
        )
        .unwrap();
    let result = engine.call(&module, "sum_to", &[Value::I32(10)]).unwrap();
    // 1+2+...+10 = 55
    assert_eq!(result, Value::I32(55));
}

#[test]
fn engine_compile_and_call_while_loop() {
    let engine = Engine::new();
    let module = engine
        .compile(
            "test",
            r#"
        int countdown(int start) {
            int count = 0;
            while (start > 0) {
                count = count + 1;
                start = start - 1;
            }
            return count;
        }
    "#,
        )
        .unwrap();
    let result = engine.call(&module, "countdown", &[Value::I32(5)]).unwrap();
    assert_eq!(result, Value::I32(5));
}

#[test]
fn engine_compile_multiple_functions() {
    let engine = Engine::new();
    let module = engine
        .compile(
            "test",
            r#"
        int twice(int x) { return x + x; }
        int thrice(int x) { return x + x + x; }
    "#,
        )
        .unwrap();

    let result = engine.call(&module, "twice", &[Value::I32(5)]).unwrap();
    assert_eq!(result, Value::I32(10));

    let result = engine.call(&module, "thrice", &[Value::I32(5)]).unwrap();
    assert_eq!(result, Value::I32(15));
}

#[test]
fn engine_function_not_found() {
    let engine = Engine::new();
    let module = engine
        .compile(
            "test",
            r#"
        int foo() { return 1; }
    "#,
        )
        .unwrap();
    let result = engine.call(&module, "bar", &[]);
    assert!(result.is_err());
}

#[test]
fn engine_compile_error() {
    let engine = Engine::new();
    // Missing semicolons / invalid syntax should fail.
    let result = engine.compile("test", "int foo( { return 1 }");
    assert!(result.is_err());
    if let Err(EngineError::CompileError(diags)) = result {
        assert!(!diags.is_empty());
    }
}

#[test]
fn engine_negation() {
    let engine = Engine::new();
    let module = engine
        .compile(
            "test",
            r#"
        int neg(int x) { return -x; }
    "#,
        )
        .unwrap();
    let result = engine.call(&module, "neg", &[Value::I32(42)]).unwrap();
    assert_eq!(result, Value::I32(-42));
}
