use angelscript::prelude::{
    Behaviour, ContextState, Engine, GetModuleFlags, ObjectTypeFlags, ReturnCode, ScriptError,
    ScriptGeneric, ScriptResult, TypeId, TypeModifiers,
};
use std::collections::HashMap;
use angelscript_core::types::script_memory::Void;

#[repr(C)]
#[derive(Debug, Clone)]
struct ComplexPlayer {
    health: i32,
    position: (f32, f32, f32),
    name: String,
    inventory: Vec<String>,
    metadata: HashMap<String, i32>,
}

impl ComplexPlayer {
    fn new() -> Self {
        Self {
            health: 100,
            position: (0.0, 0.0, 0.0),
            name: String::from("New Player"),
            inventory: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

// Constructor/destructor functions
fn construct_complex_player(g: &ScriptGeneric) {
    let mut ptr = g.get_object().unwrap();
    ptr.set(ComplexPlayer::new());
    println!("✅ Created ComplexPlayer");
}

fn destruct_complex_player(g: &ScriptGeneric) {
    let mut obj = g.get_object().unwrap();
    let player = obj.as_ref_mut::<ComplexPlayer>();

    println!("🗑️  Destroying player: {}", player.name);

    unsafe {
        std::ptr::drop_in_place(player);
    }
}

// Method implementations
fn player_get_name(g: &ScriptGeneric) {
    let obj = g.get_object().unwrap();
    let player = obj.as_ref::<ComplexPlayer>();

    let mut return_location = g.get_address_of_return_location().unwrap();
    return_location.set(player.name.clone());
}

fn player_set_name(g: &ScriptGeneric) {
    let mut obj = g.get_object().unwrap();
    let new_name_ptr = g.get_arg_address(0).unwrap();
    let new_name = new_name_ptr.as_ref::<String>();

    let player = obj.as_ref_mut::<ComplexPlayer>();
    println!("📝 Changing name from '{}' to '{}'", player.name, new_name);
    player.name = new_name.clone();
}

fn player_add_item(g: &ScriptGeneric) {
    let mut obj = g.get_object().unwrap();
    let item_ptr = g.get_arg_address(0).unwrap();
    let item = item_ptr.as_ref::<String>();

    let player = obj.as_ref_mut::<ComplexPlayer>();
    player.inventory.push(item.clone());
    println!(
        "🎒 Added '{}' to inventory (size: {})",
        item,
        player.inventory.len()
    );
}

fn player_get_inventory_size(g: &ScriptGeneric) {
    let obj = g.get_object().unwrap();
    let player = obj.as_ref::<ComplexPlayer>();

    g.set_return_dword(player.inventory.len() as u32).unwrap();
}

fn setup_engine() -> ScriptResult<Engine> {
    let mut engine = Engine::create()?;

    engine.install(angelscript::addons::string::addon())?;

    // Set up message callback
    engine.set_message_callback(
        |msg, _| {
            println!(
                "[{:?}] {} {} {} - {}",
                msg.msg_type, msg.row, msg.col, msg.section, msg.message
            );
        },
        None,
    )?;

    Ok(engine)
}

fn register_complex_player(engine: &Engine) -> ScriptResult<()> {
    engine.register_object_type(
        "ComplexPlayer",
        std::mem::size_of::<ComplexPlayer>() as i32,
        ObjectTypeFlags::VALUE | ObjectTypeFlags::APP_CLASS_CDAK,
    )?;

    engine.register_object_behaviour(
        "ComplexPlayer",
        Behaviour::Construct,
        "void f()",
        construct_complex_player,
        None,
        None,
        None,
    )?;

    engine.register_object_behaviour(
        "ComplexPlayer",
        Behaviour::Destruct,
        "void f()",
        destruct_complex_player,
        None,
        None,
        None,
    )?;

    engine.register_object_method(
        "ComplexPlayer",
        "string get_name()",
        player_get_name,
        None,
        None,
        None,
    )?;

    engine.register_object_method(
        "ComplexPlayer",
        "void set_name(const string &in)",
        player_set_name,
        None,
        None,
        None,
    )?;

    engine.register_object_method(
        "ComplexPlayer",
        "void add_item(const string &in)",
        player_add_item,
        None,
        None,
        None,
    )?;

    engine.register_object_method(
        "ComplexPlayer",
        "uint get_inventory_size() const",
        player_get_inventory_size,
        None,
        None,
        None,
    )?;

    Ok(())
}

fn print(g: &ScriptGeneric) {
    let (type_id, flags) = g.get_arg_type_id(0);

    if let Some(output) = format_primitive_type(g, type_id) {
        println!("{}", output);
    } else if type_id == TypeId::MaskObject {
        print_object_type(g, type_id, &flags);
    } else if type_id == TypeId::Void {
        println!("void");
    } else {
        println!("Unknown type (ID: {:?}, flags: {:?})", type_id, flags);
    }
}

fn format_primitive_type(g: &ScriptGeneric, type_id: TypeId) -> Option<String> {
    let value_ptr = g.get_arg_address(0)?;

    if type_id == TypeId::Bool {
        let value = value_ptr.as_ref::<bool>();
        Some(if *value {
            "true".to_string()
        } else {
            "false".to_string()
        })
    } else if type_id == TypeId::Int8 {
        Some(value_ptr.as_ref::<i8>().to_string())
    } else if type_id == TypeId::Int16 {
        Some(value_ptr.as_ref::<i16>().to_string())
    } else if type_id == TypeId::Int32 {
        Some(value_ptr.as_ref::<i32>().to_string())
    } else if type_id == TypeId::Int64 {
        Some(value_ptr.as_ref::<i64>().to_string())
    } else if type_id == TypeId::Uint8 {
        Some(value_ptr.as_ref::<u8>().to_string())
    } else if type_id == TypeId::Uint16 {
        Some(value_ptr.as_ref::<u16>().to_string())
    } else if type_id == TypeId::Uint32 {
        Some(value_ptr.as_ref::<u32>().to_string())
    } else if type_id == TypeId::Uint64 {
        Some(value_ptr.as_ref::<u64>().to_string())
    } else if type_id == TypeId::Float {
        Some(value_ptr.as_ref::<f32>().to_string())
    } else if type_id == TypeId::Double {
        Some(value_ptr.as_ref::<f64>().to_string())
    } else {
        None
    }
}

fn print_object_type(g: &ScriptGeneric, type_id: TypeId, _flags: &TypeModifiers) {
    let engine = g.get_engine().unwrap();

    if let Some(type_info) = engine.get_type_info_by_id(type_id) {
        let type_name = type_info.get_name();

        match type_name {
            Some("string") => print_string_type(g, type_id),
            Some("ComplexPlayer") => print_complex_player(g),
            Some(ty) => println!("Object of type: {}", ty),
            None => println!("Unknown object type (ID: {:?})", type_id),
        }
    } else {
        println!("Unknown object type (ID: {:?})", type_id);
    }
}

fn print_string_type(g: &ScriptGeneric, type_id: TypeId) {
    if type_id == TypeId::ObjHandle {
        // Handle to string (string@)
        if let Some(handle_ptr) = g.get_arg_object(0) {
            if !handle_ptr.is_null() {
                let string_value = handle_ptr.as_ref::<String>();
                println!("{}", string_value);
            } else {
                println!("null");
            }
        }
    } else {
        // Value string (const string&)
        if let Some(value_ptr) = g.get_arg_address(0) {
            let string_value = value_ptr.as_ref::<String>();
            println!("{}", string_value);
        }
    }
}

fn print_complex_player(g: &ScriptGeneric) {
    if let Some(value_ptr) = g.get_arg_address(0) {
        let player = value_ptr.as_ref::<ComplexPlayer>();
        println!(
            "ComplexPlayer {{ name: \"{}\", health: {} }}",
            player.name, player.health
        );
    }
}

fn main() -> ScriptResult<()> {
    println!("🚀 Setting up AngelScript with unified memory management");

    let engine = setup_engine()?;
    register_complex_player(&engine)?;

    println!("📜 Running test script");

    let script = r#"
        void main() {
            ComplexPlayer player;
            print(player.get_name());
            player.set_name("Hero");
            print(player.get_name());
            player.add_item("Sword");
            player.add_item("Shield");
            player.add_item("Health Potion");
            print(player.get_inventory_size());
        }
    "#;

    engine.register_global_function("void print(const ?&in)", print, None)?;

    // Create a module
    let module = engine.get_module("MyModule", GetModuleFlags::AlwaysCreate)?;
    module.add_script_section("main", script, 0)?;
    module.build()?;

    let func = module.get_function_by_name("main").unwrap();

    let ctx = engine.create_context()?;
    ctx.prepare(&func)?;
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
