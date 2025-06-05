use angelscript::core::engine::Engine;
use angelscript::prelude::{GetModuleFlags, ScriptGeneric};

fn print(g: &ScriptGeneric) {
    let arg_ptr = g.get_arg_object::<String>(0).unwrap();
    println!("Hello, {}", arg_ptr.as_ref());
}

fn main() {
    // Create the script engine
    let mut engine = Engine::create().expect("Failed to create script engine");

    // Set up message callback
    engine
        .set_message_callback(|msg| {
            println!("AngelScript: {}", msg.message);
        })
        .expect("Failed to set message callback");

    engine
        .with_default_modules()
        .expect("Failed to register std");

    engine
        .register_global_function("void print(const string &in)", print, None)
        .unwrap();

    // Create a module
    let module = engine
        .get_module("MyModule", GetModuleFlags::AlwaysCreate)
        .expect("Failed to create module");

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

    module
        .add_script_section_simple("main", script)
        .expect("Failed to add script");

    // Build the module
    module.build().expect("Failed to build module");

    let ctx = engine.create_context().expect("Failed to create context");

    let main_func = module
        .get_function_by_decl("void main()")
        .expect("Failed to find main function");

    // Create a context and execute
    ctx.prepare(&main_func).expect("Failed to prepare context");
    ctx.execute().expect("Failed to execute script");

    let print_func = module
        .get_function_by_name("say")
        .expect("Failed to find print function");

    let mut name = "Cat".to_string();

    ctx.prepare(&print_func).expect("Failed to prepare context");
    ctx.set_arg_object(0, &mut name)
        .expect("Failed to bind str");
    ctx.execute().expect("Failed to execute script");

    println!("Script execution completed!");
}
