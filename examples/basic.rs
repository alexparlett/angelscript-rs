use angelscript::prelude::{
    ContextState, GetModuleFlags, ReturnCode, ScriptError, ScriptGeneric, ScriptResult,
};
use angelscript_core::core::engine::Engine;

fn print(g: &ScriptGeneric) {
    let arg_ptr = g.get_arg_object(0).unwrap();
    println!("Hello, {}", arg_ptr.as_ref::<String>());
}

fn main() -> ScriptResult<()> {
    // Create the script engine
    let mut engine = Engine::create().expect("Failed to create script engine");

    // Set up message callback
    engine.set_message_callback(|msg| {
        println!("AngelScript: {}", msg.message);
    })?;

    engine.install(angelscript::addons::string::addon())?;

    engine.register_global_function("void print(const string &in)", print, None)?;

    // Create a module
    let module = engine.get_module("MyModule", GetModuleFlags::AlwaysCreate)?;

    // Add a simple script without strings for now
    let script = r#"
        void say(const string &in msg) {
            print(msg);
        }
    
        void main() {
            int x = 5;
            int y = 10;
            int result = x + y;
            print("carl");
        }
    "#;

    module.add_script_section_simple("main", script)?;

    // Build the module
    module.build()?;

    let ctx = engine.create_context()?;

    let main_func = module.get_function_by_decl("void main()").unwrap();

    // Create a context and execute
    ctx.prepare(&main_func)?;
    ctx.execute()?;

    let print_func = module.get_function_by_name("say").unwrap();

    let mut name = "Cat".to_string();

    ctx.prepare(&print_func)?;
    ctx.set_arg_object(0, &mut name)?;
    let result = ctx.execute()?;

    if result == ContextState::Exception {
        let error = ctx.get_exception_string();
        let (line, column, section) = ctx.get_exception_line_number();
        println!(
            "Exception {:?} at {}, {:?}, {:?}",
            error, line, column, section
        );

        println!("Script execution failed");

        Err(ScriptError::AngelScriptError(ReturnCode::Error))
    } else {
        println!("✅ Script execution completed");

        Ok(())
    }
}
