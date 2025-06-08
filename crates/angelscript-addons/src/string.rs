use crate::stringfactory::get_string_factory_instance;
use crate::Addon;
use angelscript_core::core::engine::Engine;
use angelscript_core::core::script_generic::ScriptGeneric;
use angelscript_core::types::enums::{Behaviour, ObjectTypeFlags};
use angelscript_core::types::script_memory::ScriptMemoryLocation;
use angelscript_sys::{asINT64, asUINT};

// Constructors
fn construct_string(g: &ScriptGeneric) {
    let mut ptr = g.get_object().unwrap();
    ptr.set(String::new());
}

fn copy_construct_string(g: &ScriptGeneric) {
    let src_ptr = g.get_arg_object(0).unwrap();
    let mut dest_ptr = g.get_object().unwrap();
    if src_ptr.is_null() {
        return;
    }
    let source_value: String = src_ptr.as_ref::<String>().to_string();
    dest_ptr.set(source_value);
}

// Destructor
fn destruct_string(g: &ScriptGeneric) {
    let mut ptr = g.get_object().unwrap();
    ptr.as_ref_mut::<String>().clear();
}

// opAssign functions
fn string_assign(g: &ScriptGeneric) {
    let src = g.get_arg_address(0).unwrap();
    let mut dest = g.get_object().unwrap();
    if src.is_null() {
        return;
    }
    dest.set(src.as_ref::<String>().clone());
    g.set_return_address_raw(dest)
        .expect("Failed to return string");
}

fn string_assign_int(g: &ScriptGeneric) {
    let src = g.get_address_of_arg(0).unwrap();
    let mut dest = g.get_object().unwrap();
    if src.is_null() {
        return;
    }
    dest.set(src.as_ref::<asINT64>().to_string());
    g.set_return_address_raw(dest)
        .expect("Failed to return string");
}

fn string_assign_uint(g: &ScriptGeneric) {
    let src = g.get_address_of_arg(0).unwrap();
    let mut dest = g.get_object().unwrap();
    if src.is_null() {
        return;
    }
    dest.set(src.as_ref::<asUINT>().to_string());
    g.set_return_address_raw(dest)
        .expect("Failed to return string");
}

fn string_assign_double(g: &ScriptGeneric) {
    let src = g.get_address_of_arg(0).unwrap();
    let mut dest = g.get_object().unwrap();
    if src.is_null() {
        return;
    }
    dest.set(src.as_ref::<f64>().to_string());
    g.set_return_address_raw(dest)
        .expect("Failed to return string");
}

fn string_assign_float(g: &ScriptGeneric) {
    let src = g.get_address_of_arg(0).unwrap();
    let mut dest = g.get_object().unwrap();
    if src.is_null() {
        return;
    }
    dest.set(src.as_ref::<f32>().to_string());
    g.set_return_address_raw(dest)
        .expect("Failed to return string");
}

fn string_assign_bool(g: &ScriptGeneric) {
    let src = g.get_address_of_arg(0).unwrap();
    let mut dest = g.get_object().unwrap();
    if src.is_null() {
        return;
    }
    dest.set(src.as_ref::<bool>().to_string());
    g.set_return_address_raw(dest)
        .expect("Failed to return string");
}

// opAddAssign functions
fn add_assign_string(g: &ScriptGeneric) {
    let src = g.get_arg_address(0).unwrap();
    let mut dest = g.get_object().unwrap();
    if src.is_null() {
        return;
    }
    dest.as_ref_mut::<String>().push_str(src.as_ref::<String>());
    g.set_return_object(&mut dest)
        .expect("Failed to return string");
}

fn string_add_assign_double(g: &ScriptGeneric) {
    let src = g.get_arg_address(0).unwrap();
    let mut dest = g.get_object().unwrap();
    if src.is_null() {
        return;
    }
    dest.as_ref_mut::<String>()
        .push_str(src.as_ref::<f64>().to_string().as_str());
    g.set_return_object(&mut dest)
        .expect("Failed to return string");
}

fn string_add_assign_float(g: &ScriptGeneric) {
    let src = g.get_address_of_arg(0).unwrap();
    let mut dest = g.get_object().unwrap();
    if src.is_null() {
        return;
    }
    dest.as_ref_mut::<String>()
        .push_str(src.as_ref::<f32>().to_string().as_str());
    g.set_return_object(&mut dest)
        .expect("Failed to return string");
}

fn string_add_assign_int(g: &ScriptGeneric) {
    let src = g.get_address_of_arg(0).unwrap();
    let mut dest = g.get_object().unwrap();
    if src.is_null() {
        return;
    }
    dest.as_ref_mut::<String>()
        .push_str(src.as_ref::<i64>().to_string().as_str());
    g.set_return_object(&mut dest)
        .expect("Failed to return string");
}

fn string_add_assign_uint(g: &ScriptGeneric) {
    let src = g.get_address_of_arg(0).unwrap();
    let mut dest = g.get_object().unwrap();
    if src.is_null() {
        return;
    }
    dest.as_ref_mut::<String>()
        .push_str(src.as_ref::<u64>().to_string().as_str());
    g.set_return_object(&mut dest)
        .expect("Failed to return string");
}

fn string_add_assign_bool(g: &ScriptGeneric) {
    let src = g.get_address_of_arg(0).unwrap();
    let mut dest = g.get_object().unwrap();
    if src.is_null() {
        return;
    }
    dest.as_ref_mut::<String>()
        .push_str(src.as_ref::<bool>().to_string().as_str());
    g.set_return_object(&mut dest)
        .expect("Failed to return string");
}

// opAdd functions
fn string_add(g: &ScriptGeneric) {
    let lhs_ptr = g.get_object().unwrap();
    let rhs_ptr = g.get_arg_address(0).unwrap();
    let mut buffer = String::new();
    if !lhs_ptr.is_null() {
        buffer.push_str(lhs_ptr.as_ref::<String>());
    }
    if !rhs_ptr.is_null() {
        buffer.push_str(rhs_ptr.as_ref::<String>());
    }
    g.set_return_object::<String>(&mut buffer).unwrap();
}

fn string_add_double(g: &ScriptGeneric) {
    let lhs_ptr = g.get_object().unwrap();
    let rhs_ptr = g.get_arg_address(0).unwrap();
    let mut buffer = String::new();
    if !lhs_ptr.is_null() {
        buffer.push_str(lhs_ptr.as_ref::<String>());
    }
    if !rhs_ptr.is_null() {
        buffer.push_str(rhs_ptr.as_ref::<f64>().to_string().as_str());
    }
    g.set_return_object::<String>(&mut buffer).unwrap();
}

fn double_add_string(g: &ScriptGeneric) {
    let lhs_ptr = g.get_address_of_arg(0).unwrap();
    let rhs_ptr = g.get_object().unwrap();
    let mut buffer = String::new();
    if !lhs_ptr.is_null() {
        buffer.push_str(lhs_ptr.as_ref::<f64>().to_string().as_str());
    }
    if !rhs_ptr.is_null() {
        buffer.push_str(rhs_ptr.as_ref::<String>());
    }
    g.set_return_object::<String>(&mut buffer).unwrap();
}

fn string_add_float(g: &ScriptGeneric) {
    let lhs_ptr = g.get_object().unwrap();
    let rhs_ptr = g.get_arg_address(0).unwrap();
    let mut buffer = String::new();
    if !lhs_ptr.is_null() {
        buffer.push_str(lhs_ptr.as_ref::<String>());
    }
    if !rhs_ptr.is_null() {
        buffer.push_str(rhs_ptr.as_ref::<f32>().to_string().as_str());
    }
    g.set_return_object::<String>(&mut buffer).unwrap();
}

fn float_add_string(g: &ScriptGeneric) {
    let lhs_ptr = g.get_address_of_arg(0).unwrap();
    let rhs_ptr = g.get_object().unwrap();
    let mut buffer = String::new();
    if !lhs_ptr.is_null() {
        buffer.push_str(lhs_ptr.as_ref::<f32>().to_string().as_str());
    }
    if !rhs_ptr.is_null() {
        buffer.push_str(rhs_ptr.as_ref::<String>());
    }
    g.set_return_object::<String>(&mut buffer).unwrap();
}

fn string_add_int(g: &ScriptGeneric) {
    let lhs_ptr = g.get_object().unwrap();
    let rhs_ptr = g.get_arg_address(0).unwrap();
    let mut buffer = String::new();
    if !lhs_ptr.is_null() {
        buffer.push_str(lhs_ptr.as_ref::<String>());
    }
    if !rhs_ptr.is_null() {
        buffer.push_str(rhs_ptr.as_ref::<i64>().to_string().as_str());
    }
    g.set_return_object::<String>(&mut buffer).unwrap();
}

fn int_add_string(g: &ScriptGeneric) {
    let lhs_ptr = g.get_address_of_arg(0).unwrap();
    let rhs_ptr = g.get_object().unwrap();
    let mut buffer = String::new();
    if !lhs_ptr.is_null() {
        buffer.push_str(lhs_ptr.as_ref::<asINT64>().to_string().as_str());
    }
    if !rhs_ptr.is_null() {
        buffer.push_str(rhs_ptr.as_ref::<String>());
    }
    g.set_return_object::<String>(&mut buffer).unwrap();
}

fn string_add_uint(g: &ScriptGeneric) {
    let lhs_ptr = g.get_object().unwrap();
    let rhs_ptr = g.get_arg_address(0).unwrap();
    let mut buffer = String::new();
    if !lhs_ptr.is_null() {
        buffer.push_str(lhs_ptr.as_ref::<String>());
    }
    if !rhs_ptr.is_null() {
        buffer.push_str(rhs_ptr.as_ref::<u32>().to_string().as_str());
    }
    g.set_return_object::<String>(&mut buffer).unwrap();
}

fn uint_add_string(g: &ScriptGeneric) {
    let lhs_ptr = g.get_address_of_arg(0).unwrap();
    let rhs_ptr = g.get_object().unwrap();
    let mut buffer = String::new();
    if !lhs_ptr.is_null() {
        buffer.push_str(lhs_ptr.as_ref::<u64>().to_string().as_str());
    }
    if !rhs_ptr.is_null() {
        buffer.push_str(rhs_ptr.as_ref::<String>());
    }
    g.set_return_object::<String>(&mut buffer).unwrap();
}

fn string_add_bool(g: &ScriptGeneric) {
    let lhs_ptr = g.get_object().unwrap();
    let rhs_ptr = g.get_arg_address(0).unwrap();
    let mut buffer = String::new();
    if !lhs_ptr.is_null() {
        buffer.push_str(lhs_ptr.as_ref::<String>());
    }
    if !rhs_ptr.is_null() {
        buffer.push_str(rhs_ptr.as_ref::<bool>().to_string().as_str());
    }
    g.set_return_object::<String>(&mut buffer).unwrap();
}

fn bool_add_string(g: &ScriptGeneric) {
    let lhs_ptr = g.get_address_of_arg(0).unwrap();
    let rhs_ptr = g.get_object().unwrap();
    let mut buffer = String::new();
    if !lhs_ptr.is_null() {
        buffer.push_str(lhs_ptr.as_ref::<bool>().to_string().as_str());
    }
    if !rhs_ptr.is_null() {
        buffer.push_str(rhs_ptr.as_ref::<String>());
    }
    g.set_return_object::<String>(&mut buffer).unwrap();
}

// Comparison operators
fn string_equals(g: &ScriptGeneric) {
    let lhs = g.get_object().unwrap();
    let rhs = g.get_arg_address(0).unwrap();
    let equal = lhs.as_ref::<String>() == rhs.as_ref::<String>();
    g.set_return_byte(equal.into()).unwrap();
}

fn string_cmp(g: &ScriptGeneric) {
    let lhs = g.get_object().unwrap();
    let rhs = g.get_arg_address(0).unwrap();
    let lhs_str = lhs.as_ref::<String>();
    let rhs_str = rhs.as_ref::<String>();
    g.set_return_dword(if lhs_str < rhs_str {
        -1
    } else if lhs_str > rhs_str {
        1
    } else {
        0
    } as u32)
        .unwrap();
}

// String operations
fn string_length(g: &ScriptGeneric) {
    let obj = g.get_object().unwrap();
    g.set_return_dword(obj.as_ref::<String>().len() as u32)
        .unwrap();
}

fn string_is_empty(g: &ScriptGeneric) {
    let obj = g.get_object().unwrap();
    g.set_return_byte(obj.as_ref::<String>().is_empty().into())
        .unwrap();
}

fn string_char_at(g: &ScriptGeneric) {
    let idx = g.get_arg_dword(0) as usize;
    let mut obj = g.get_object().unwrap();
    let str = obj.as_ref_mut::<String>();
    if idx >= str.len() {
        let ctx = Engine::get_active_context().unwrap();
        ctx.set_exception("Index out of bounds", true).unwrap();
        g.set_return_address_raw(ScriptMemoryLocation::null())
            .unwrap();
        return;
    }
    unsafe {
        g.set_return_address(str.as_bytes_mut().get_mut(idx).unwrap())
            .unwrap();
    }
}

// Substring
fn string_substring(g: &ScriptGeneric) {
    let self_ptr = g.get_object().unwrap();
    let start: i32 = g.get_address_of_arg(0).unwrap().read();
    let len: i32 = g.get_address_of_arg(1).unwrap().read();

    let source = self_ptr.as_ref::<String>();

    if start >= source.len() as i32 {
        let ctx = Engine::get_active_context().unwrap();
        ctx.set_exception("start is greater than length of string", true)
            .unwrap();
        return;
    }

    let mut substring = if len > 0 {
        if (start + len) > source.len() as i32 {
            let ctx = Engine::get_active_context().unwrap();
            ctx.set_exception("start is outside of bounds", true)
                .unwrap();
            return;
        }
        source
            .get(start as usize..(start + len) as usize)
            .unwrap_or("")
            .to_string()
    } else {
        source.get(start as usize..).unwrap_or("").to_string()
    };

    g.set_return_object::<String>(&mut substring).unwrap();
}

/// Create a string plugin for AngelScript
pub fn addon() -> Addon {
    let addon = Addon::new().ty::<String>("string", |ctx| {
        ctx.as_value_type()
            .with_flags(ObjectTypeFlags::VALUE | ObjectTypeFlags::APP_CLASS_CDAK)
            // Constructors
            .with_behavior(
                Behaviour::Construct,
                "void f()",
                construct_string,
                None,
                None,
                None,
            )
            .with_behavior(
                Behaviour::Construct,
                "void f(const string &in)",
                copy_construct_string,
                None,
                None,
                None,
            )
            .with_behavior(
                Behaviour::Destruct,
                "void f()",
                destruct_string,
                None,
                None,
                None,
            )
            // opAssign methods
            .with_method(
                "string &opAssign(const string &in)",
                string_assign,
                None,
                None,
                None,
            )
            .with_method(
                "string &opAssign(double)",
                string_assign_double,
                None,
                None,
                None,
            )
            .with_method(
                "string &opAssign(float)",
                string_assign_float,
                None,
                None,
                None,
            )
            .with_method(
                "string &opAssign(int64)",
                string_assign_int,
                None,
                None,
                None,
            )
            .with_method(
                "string &opAssign(uint64)",
                string_assign_uint,
                None,
                None,
                None,
            )
            .with_method(
                "string &opAssign(bool)",
                string_assign_bool,
                None,
                None,
                None,
            )
            // opAddAssign methods
            .with_method(
                "string &opAddAssign(const string &in)",
                add_assign_string,
                None,
                None,
                None,
            )
            .with_method(
                "string &opAddAssign(double)",
                string_add_assign_double,
                None,
                None,
                None,
            )
            .with_method(
                "string &opAddAssign(float)",
                string_add_assign_float,
                None,
                None,
                None,
            )
            .with_method(
                "string &opAddAssign(int64)",
                string_add_assign_int,
                None,
                None,
                None,
            )
            .with_method(
                "string &opAddAssign(uint64)",
                string_add_assign_uint,
                None,
                None,
                None,
            )
            .with_method(
                "string &opAddAssign(bool)",
                string_add_assign_bool,
                None,
                None,
                None,
            )
            // opAdd methods
            .with_method(
                "string opAdd(const string &in) const",
                string_add,
                None,
                None,
                None,
            )
            .with_method(
                "string opAdd(double) const",
                string_add_double,
                None,
                None,
                None,
            )
            .with_method(
                "string opAdd_r(double) const",
                double_add_string,
                None,
                None,
                None,
            )
            .with_method(
                "string opAdd(float) const",
                string_add_float,
                None,
                None,
                None,
            )
            .with_method(
                "string opAdd_r(float) const",
                float_add_string,
                None,
                None,
                None,
            )
            .with_method(
                "string opAdd(int64) const",
                string_add_int,
                None,
                None,
                None,
            )
            .with_method(
                "string opAdd_r(int64) const",
                int_add_string,
                None,
                None,
                None,
            )
            .with_method(
                "string opAdd(uint64) const",
                string_add_uint,
                None,
                None,
                None,
            )
            .with_method(
                "string opAdd_r(uint64) const",
                uint_add_string,
                None,
                None,
                None,
            )
            .with_method(
                "string opAdd(bool) const",
                string_add_bool,
                None,
                None,
                None,
            )
            .with_method(
                "string opAdd_r(bool) const",
                bool_add_string,
                None,
                None,
                None,
            )
            // Comparison operators
            .with_method(
                "bool opEquals(const string &in) const",
                string_equals,
                None,
                None,
                None,
            )
            .with_method(
                "int opCmp(const string &in) const",
                string_cmp,
                None,
                None,
                None,
            )
            // Other string methods
            .with_method("uint length() const", string_length, None, None, None)
            .with_method("bool isEmpty() const", string_is_empty, None, None, None)
            .with_method("uint8 &opIndex(uint)", string_char_at, None, None, None)
            .with_method(
                "uint8 &opIndex(uint) const",
                string_char_at,
                None,
                None,
                None,
            )
            .with_method(
                "string substr(uint start = 0, int count = -1) const",
                string_substring,
                None,
                None,
                None,
            )
            .with_engine_configuration(|e| {
                e.register_string_factory("string", get_string_factory_instance())
            });
        //     // engine.register_object_method("string", "int findFirst(const string &in, uint start = 0) const", string_find_first, asCALL_GENERIC)?;
        //     // engine.register_object_method("string", "int findFirstOf(const string &in, uint start = 0) const", string_find_first_of, asCALL_GENERIC)?;
        //     // engine.register_object_method("string", "int findFirstNotOf(const string &in, uint start = 0) const", string_find_first_not_of, asCALL_GENERIC)?;
        //     // engine.register_object_method("string", "int findLast(const string &in, int start = -1) const", string_find_last, asCALL_GENERIC)?;
        //     // engine.register_object_method("string", "int findLastOf(const string &in, int start = -1) const", string_find_last_of, asCALL_GENERIC)?;
        //     // engine.register_object_method("string", "int findLastNotOf(const string &in, int start = -1) const", string_find_last_not_of, asCALL_GENERIC)?;
        //     // engine.register_object_method("string", "void insert(uint pos, const string &in other)", string_insert, asCALL_GENERIC)?;
        //     // engine.register_object_method("string", "void erase(uint pos, int count = -1)", string_erase, asCALL_GENERIC)?;
        //     //
        //     // engine.register_global_function("string formatInt(int64 val, const string &in options = \"\", uint width = 0)", format_int, asCALL_GENERIC)?;
        //     // engine.register_global_function("string formatUInt(uint64 val, const string &in options = \"\", uint width = 0)", format_uint, asCALL_GENERIC)?;
        //     // engine.register_global_function("string formatFloat(double val, const string &in options = \"\", uint width = 0, uint precision = 0)", format_float, asCALL_GENERIC)?;
        //     // engine.register_global_function("int64 parseInt(const string &in, uint base = 10, uint &out byteCount = 0)", parse_int, asCALL_GENERIC)?;
        //     // engine.register_global_function("uint64 parseUInt(const string &in, uint base = 10, uint &out byteCount = 0)", parse_u_int, asCALL_GENERIC)?;
        //     // engine.register_global_function("double parseFloat(const string &in, uint &out byteCount = 0)", parse_float, asCALL_GENERIC)?;
    });

    addon
}
