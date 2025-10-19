use angelscript::{ExecutionResult, GetModuleFlag, ScriptEngine, TypeFlags};

fn main() {
    println!("AngelScript Basic Example\n");

    // Create engine
    let mut engine = ScriptEngine::new();

    // Register types
    engine
        .register_object_type_raw("string", 0, TypeFlags::REF_TYPE)
        .expect("Failed to register string");

    // Create a module
    let module = engine
        .get_module("MyModule", GetModuleFlag::AlwaysCreate)
        .expect("Failed to create module");

    // Add script section
    module
        .add_script_section(
            "main.as",
            r#"
        int add(int a, int b) {
            return a + b;
        }

        int multiply(int a, int b) {
            return a * b;
        }

        void main() {
            int result = add(10, 20);
        }
    "#,
        )
        .expect("Failed to add script section");

    // Build the module
    println!("Building module...");
    let r = module.build();
    if r < 0 {
        eprintln!("Build failed!");
        return;
    }

    println!("✓ Build successful!\n");

    // Create context
    let mut ctx = engine.create_context();

    // Call add(10, 20)
    println!("Calling add(10, 20)...");
    ctx.prepare(module, "add").expect("Failed to prepare");
    ctx.set_arg_dword(0, 10).expect("Failed to set arg 0");
    ctx.set_arg_dword(1, 20).expect("Failed to set arg 1");

    let result = ctx.execute().expect("Execution failed");
    if result == ExecutionResult::Finished {
        let return_value = ctx.get_return_dword().expect("Failed to get return value");
        println!("Result: {}\n", return_value);
    }

    // Reuse context for multiply(5, 6)
    ctx.unprepare();

    println!("Calling multiply(5, 6)...");
    ctx.prepare(module, "multiply").expect("Failed to prepare");
    ctx.set_arg_dword(0, 5).expect("Failed to set arg 0");
    ctx.set_arg_dword(1, 6).expect("Failed to set arg 1");

    let result = ctx.execute().expect("Execution failed");
    if result == ExecutionResult::Finished {
        let return_value = ctx.get_return_dword().expect("Failed to get return value");
        println!("Result: {}", return_value);
    }

    println!("\n✓ Example completed successfully!");
}
