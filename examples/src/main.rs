use angelscript::asIScriptGeneric;
use angelscript::FromScriptGeneric;
use angelscript::{
    CallConvTypes, Engine, GMFlags
    ,
};
use angelscript_macros::as_function;

#[as_function]
fn print(msg: &str) {
    println!("Hello {}", msg);
}

fn main() {
    // Create the script engine
    let mut engine = Engine::new().expect("Failed to create engine");

    // Set up message callback
    engine
        .set_message_callback(|msg| {
            println!("AngelScript: {}", msg.message);
        })
        .ok();

    engine.register_std();

    engine
        .register_global_function(
            "void print(const string &in)",
            print_as_generic,
            CallConvTypes::asCALL_GENERIC,
        )
        .unwrap();

    // Create a module
    let module = engine
        .get_module("MyModule", GMFlags::asGM_CREATE_IF_NOT_EXISTS)
        .expect("Failed to create module");

    // Add a simple script without strings for now
    let script = r#"
        void main() {
            int x = 5;
            int y = 10;
            int result = x + y;
            print("" + result);
        }
    "#;

    module
        .add_script_section("main", script)
        .expect("Failed to add script");

    // Build the module
    module.build().expect("Failed to build module");

    // Get the main function
    let main_func = module
        .get_function_by_decl("void main()")
        .expect("Failed to find main function");

    // Create a context and execute
    let ctx = engine.create_context().expect("Failed to create context");
    ctx.prepare(&main_func).expect("Failed to prepare context");
    ctx.execute().expect("Failed to execute script");

    println!("Script execution completed!");
}
