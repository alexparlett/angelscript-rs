use angelscript::prelude::{
    Behaviour, ContextState, Engine, GetModuleFlags, ObjectTypeFlags, ReturnCode, ScriptError,
    ScriptGeneric, ScriptResult, TypeId,
};
use std::alloc::{alloc, Layout};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use angelscript_core::types::enums::TypeModifiers;
use angelscript_core::types::script_memory::ScriptMemoryLocation;

// Reference type - same as before
#[repr(C)]
#[derive(Debug)]
struct ComplexEntity {
    ref_count: AtomicUsize,
    gc_flag: bool,
    id: u32,
    name: String,
    inventory: Vec<String>,
    metadata: HashMap<String, i32>,
    position: (f32, f32, f32),
}

impl ComplexEntity {
    fn new(name: String) -> Self {
        Self {
            ref_count: AtomicUsize::new(1),
            gc_flag: false,
            id: 1,
            name,
            inventory: Vec::new(),
            metadata: HashMap::new(),
            position: (0.0, 0.0, 0.0),
        }
    }
}

// Factory functions - MUCH CLEANER
fn entity_factory(g: &ScriptGeneric) {
    let entity = ComplexEntity::new("New Entity".to_string());
    println!("🏭 Factory: Created entity");

    // Box handling is hidden in ScriptMemoryLocation
    let handle = ScriptMemoryLocation::from_boxed(entity);
    g.set_return_address_raw(handle).unwrap();
}

fn entity_factory_with_name(g: &ScriptGeneric) {
    let name_ptr = g.get_arg_address(0).unwrap();
    let name = name_ptr.as_ref::<String>();

    let entity = ComplexEntity::new(name.clone());
    println!("🏭 Factory with name: Created '{}'", name);

    // Box handling is hidden
    let handle = ScriptMemoryLocation::from_boxed(entity);
    g.set_return_address_raw(handle).unwrap();
}

// Reference counting - MUCH CLEANER
fn entity_addref(g: &ScriptGeneric) {
    let obj = g.get_object().unwrap();
    let entity = obj.as_boxed_ref::<ComplexEntity>();
    let count = obj.addref_boxed(&entity.ref_count);
    println!("🔗 AddRef: {} refs", count);
}

fn entity_release(g: &ScriptGeneric) {
    let obj = g.get_object().unwrap();
    let entity = obj.as_boxed_ref::<ComplexEntity>();

    unsafe {
        let was_freed = obj.release_boxed::<ComplexEntity>(&entity.ref_count);
        if was_freed {
            println!("🗑️  Deleted entity");
        } else {
            let count = entity.ref_count.load(Ordering::Relaxed);
            println!("🔓 Release: {} refs remaining", count);
        }
    }
}

// Method implementations - CLEANER
fn entity_get_name(g: &ScriptGeneric) {
    let obj = g.get_object().unwrap();
    let entity = obj.as_boxed_ref::<ComplexEntity>();

    let mut return_location = g.get_address_of_return_location().unwrap();
    return_location.set(entity.name.clone());
    println!("📖 get_name: '{}'", entity.name);
}

fn entity_set_name(g: &ScriptGeneric) {
    let mut obj = g.get_object().unwrap();
    let new_name_ptr = g.get_arg_address(0).unwrap();
    let new_name = new_name_ptr.as_ref::<String>();

    let entity = obj.as_boxed_ref_mut::<ComplexEntity>();
    println!("📝 Changing name from '{}' to '{}'", entity.name, new_name);
    entity.name = new_name.clone();
}

fn entity_add_item(g: &ScriptGeneric) {
    let mut obj = g.get_object().unwrap();
    let item_ptr = g.get_arg_address(0).unwrap();
    let item = item_ptr.as_ref::<String>();

    let entity = obj.as_boxed_ref_mut::<ComplexEntity>();
    entity.inventory.push(item.clone());
    println!(
        "🎒 Added '{}' to inventory (size: {})",
        item,
        entity.inventory.len()
    );
}

fn entity_get_inventory_size(g: &ScriptGeneric) {
    let obj = g.get_object().unwrap();
    let entity = obj.as_boxed_ref::<ComplexEntity>();

    let size = entity.inventory.len() as u32;
    println!("📦 get_inventory_size: {}", size);
    g.set_return_dword(size).unwrap();
}

fn entity_get_id(g: &ScriptGeneric) {
    let obj = g.get_object().unwrap();
    let entity = obj.as_boxed_ref::<ComplexEntity>();

    println!("🆔 get_id: {}", entity.id);
    g.set_return_dword(entity.id).unwrap();
}

fn entity_set_position(g: &ScriptGeneric) {
    let mut obj = g.get_object().unwrap();
    let x = g.get_arg_float(0);
    let y = g.get_arg_float(1);
    let z = g.get_arg_float(2);

    let entity = obj.as_boxed_ref_mut::<ComplexEntity>();
    entity.position = (x, y, z);
    println!("📍 set_position: ({}, {}, {})", x, y, z);
}

fn entity_get_position_x(g: &ScriptGeneric) {
    let obj = g.get_object().unwrap();
    let entity = obj.as_boxed_ref::<ComplexEntity>();

    g.set_return_float(entity.position.0).unwrap();
}

fn register_complex_entity(engine: &Engine) -> ScriptResult<()> {
    engine.register_object_type("ComplexEntity", 0, ObjectTypeFlags::REF)?;

    // Factory functions
    engine.register_object_behaviour(
        "ComplexEntity",
        Behaviour::Factory,
        "ComplexEntity@ f()",
        entity_factory,
        None,
        None,
        None,
    )?;

    engine.register_object_behaviour(
        "ComplexEntity",
        Behaviour::Factory,
        "ComplexEntity@ f(const string &in)",
        entity_factory_with_name,
        None,
        None,
        None,
    )?;

    // Reference counting
    engine.register_object_behaviour(
        "ComplexEntity",
        Behaviour::AddRef,
        "void f()",
        entity_addref,
        None,
        None,
        None,
    )?;

    engine.register_object_behaviour(
        "ComplexEntity",
        Behaviour::Release,
        "void f()",
        entity_release,
        None,
        None,
        None,
    )?;

    // Methods
    engine.register_object_method(
        "ComplexEntity",
        "string get_name()",
        entity_get_name,
        None,
        None,
        None,
    )?;

    engine.register_object_method(
        "ComplexEntity",
        "void set_name(const string &in)",
        entity_set_name,
        None,
        None,
        None,
    )?;

    engine.register_object_method(
        "ComplexEntity",
        "uint get_id()", // ← This was missing
        entity_get_id,
        None,
        None,
        None,
    )?;

    engine.register_object_method(
        "ComplexEntity",
        "void add_item(const string &in)",
        entity_add_item,
        None,
        None,
        None,
    )?;

    engine.register_object_method(
        "ComplexEntity",
        "uint get_inventory_size()",
        entity_get_inventory_size,
        None,
        None,
        None,
    )?;

    engine.register_object_method(
        "ComplexEntity",
        "void set_position(float, float, float)", // ← This was missing
        entity_set_position,
        None,
        None,
        None,
    )?;

    engine.register_object_method(
        "ComplexEntity",
        "float get_x()", // ← This was missing
        entity_get_position_x,
        None,
        None,
        None,
    )?;

    Ok(())
}

fn setup_engine() -> ScriptResult<Engine> {
    // Set AngelScript to use our unified allocator
    let mut engine = Engine::create()?;
    engine.install(angelscript::addons::string::addon())?;

    // Set up message callback
    engine.set_message_callback(|msg| {
        println!(
            "[{:?}] {} {} {} - {}",
            msg.msg_type, msg.row, msg.col, msg.section, msg.message
        );
    })?;

    Ok(engine)
}

// Print functions (same as your existing ones)
fn print(g: &ScriptGeneric) {
    let (type_id, flags) = g.get_arg_type_id(0);

    if let Some(output) = format_primitive_type(g, type_id) {
        println!("{}", output);
    } else if type_id == TypeId::MaskObject {
        print_object_type(g, type_id, &flags);
    } else if type_id == TypeId::Void {
        println!("void");
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
            Some("ComplexPlayer") => print_complex_entity(g, type_id),
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

fn print_complex_entity(g: &ScriptGeneric, type_id: TypeId) {
    if type_id == TypeId::ObjHandle {
        // Handle to entity (ComplexEntity@)
        if let Some(handle_ptr) = g.get_arg_object(0) {
            if !handle_ptr.is_null() {
                let entity = handle_ptr.as_ref::<ComplexEntity>();
                println!(
                    "ComplexEntity@ {{ name: \"{}\", id: {} }}",
                    entity.name, entity.id
                );
            } else {
                println!("null");
            }
        }
    } else {
        // Reference to entity (const ComplexEntity&)
        if let Some(value_ptr) = g.get_arg_address(0) {
            let entity = value_ptr.as_ref::<ComplexEntity>();
            println!(
                "ComplexEntity& {{ name: \"{}\", id: {} }}",
                entity.name, entity.id
            );
        }
    }
}

fn main() -> ScriptResult<()> {
    println!("🚀 Setting up AngelScript with reference types");

    let engine = setup_engine()?;
    register_complex_entity(&engine)?;

    println!("📜 Running test script");

    let script = r#"
        void main() {
            // Create entities using different factory methods
            ComplexEntity@ entity1 = ComplexEntity();
            ComplexEntity@ entity2 = ComplexEntity("Named Entity");

            print(entity1.get_name());
            print(entity2.get_name());

            // Test reference counting
            ComplexEntity@ entity3 = entity1; // Should increment ref count

            // Modify entities
            entity1.set_name("Hero");
            entity1.add_item("Sword");
            entity1.add_item("Shield");
            entity1.set_position(10.0, 20.0, 30.0);

            print("Modified entity 1:");
            print(entity1.get_name());
            print(entity1.get_inventory_size());
            print(entity1.get_x());
            print(entity1.get_id());

            // Print the objects directly
            print(entity1);
            print(entity2);
        }
    "#;

    engine.register_global_function("void print(const ?&in)", print, None)?;

    // Create a module
    let module = engine.get_module("MyModule", GetModuleFlags::AlwaysCreate)?;
    module.add_script_section_simple("main", script)?;
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
