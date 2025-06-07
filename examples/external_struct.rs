use std::mem::offset_of;
use angelscript::prelude::{Behaviour, Engine, GetModuleFlags, ObjectTypeFlags, Plugin, ScriptGeneric};

#[repr(C)]
#[derive(Debug)]
struct Player {
    health: i32,

    position: f32,

    // This field should not be included
    internal_data: u64,
}

impl Player {
    pub fn new() -> Self {
        Self {
            health: 100,
            position: 0.0,
            internal_data: 0,
        }
    }
    
    pub fn is_alive(&self) -> bool {
        self.health > 0
    }
}

fn print(g: &ScriptGeneric) {
    let arg_ptr = g.get_arg_object(0).unwrap();
    println!("Hello, {}", arg_ptr.as_ref::<String>());
}

fn main() {
    // Create the script engine
    // Create the script engine
    let mut engine = Engine::create().expect("Failed to create script engine");

    // Set up message callback
    engine
        .set_message_callback(|msg| {
            println!("AngelScript: {}", msg.message);
        })
        .expect("Failed to set message callback");

    engine
        .with_default_plugins()
        .expect("Failed to register std");

    let plugin = Plugin::new().ty::<Player>("Player", |ctx| {
        ctx.with_flags(ObjectTypeFlags::REF | ObjectTypeFlags::NOCOUNT);
        ctx.with_property("int health", offset_of!(Player, health) as i32, None, None);
        ctx.with_property("float position", offset_of!(Player, position) as i32, None, None);

        ctx.with_behavior(Behaviour::Factory, "Player@ f()", |g| {
            let mut ret = g.get_address_of_return_location().unwrap();
            ret.set(Player::new());
        }, None, None, None);
    });
    engine.install(plugin).expect("Failed to install plugin");

    engine
        .register_global_function("void print(const string &in)", print, None)
        .unwrap();

    // Create a module
    let module = engine
        .get_module("MyModule", GetModuleFlags::AlwaysCreate)
        .expect("Failed to create module");

    // Add a simple script without strings for now
    let script = r#"
        float create_player() {
            print("=== AngelScript Debug ===");
            print("About to create Player");
            Player @x = Player();
            return x.position;
        }
    "#;

    module
        .add_script_section_simple("main", script)
        .expect("Failed to add script");

    // Build the module
    module.build().expect("Failed to build module");

    let ctx = engine.create_context().expect("Failed to create context");

    let func = module
        .get_function_by_name("create_player")
        .expect("Failed to find function");

    ctx.prepare(&func).expect("Failed to prepare context");
    ctx.execute().expect("Failed to execute script");
}
