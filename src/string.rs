use crate::stringfactory::get_string_factory_instance;
use crate::{AngelScript, Behaviour, Engine, ObjectTypeFlags, Ptr, ScriptGeneric, VoidPtr};
use angelscript_bindings::{asINT64, asIStringFactory, asQWORD, asUINT};
use std::ffi::c_void;

// Constructor from &str (used from AngelScript generics)
fn construct_string(g: &ScriptGeneric) {
    // Get the memory location for the string and set the value
    let mut ptr = g.get_object::<String>().unwrap();

    eprintln!("[String::construct_string] Pointer {:p}.", ptr.as_ptr());

    ptr.set(String::new());

    // Log the value of the pointer at the end of the method
    eprintln!(
        "[String::construct_string] Final value of pointer {:p}: value = {:?}.",
        ptr.as_ptr(),
        ptr.as_ref()
    );
}

// Copy constructor for Ustr "string"
fn copy_construct_string(g: &ScriptGeneric) {
    let src_ptr = g.get_arg_object::<String>(0).unwrap();
    let mut dest_ptr = g.get_object::<String>().unwrap();

    // Log pointers involved in the operation
    eprintln!(
        "[String::copy_construct_string] Copy from {:p} to {:p}.",
        src_ptr.as_ptr(),
        dest_ptr.as_ptr()
    );

    // Ensure the source pointer is valid
    if src_ptr.is_null() {
        eprintln!("[String::copy_construct_string] Source pointer is null. Aborting copy.");
        return;
    }

    // Correctly copy the value from `src_ptr` to `dest_ptr`.
    let source_value: String = src_ptr.as_ref().to_string();
    dest_ptr.set(source_value);

    // Log the final state after copying.
    eprintln!(
        "[String::copy_construct_string] Final values - Source Pointer {:p}: value = {:?}, Destination Pointer {:p}: value = {:?}.",
        src_ptr.as_ptr(),
        src_ptr.as_ref(),
        dest_ptr.as_ptr(),
        dest_ptr.as_ref()
    );
}

// Destructor: free the storage.
fn destruct_string(g: &ScriptGeneric) {
    let mut ptr = g.get_object::<String>().unwrap();

    eprintln!("[String::destruct_string] Pointer {:p}.", ptr.as_ptr());

    ptr.as_ref_mut().clear();

    // Log pointer after drop (note: the pointer is still valid, but memory has been dropped)
    eprintln!(
        "[String::destruct_string] After drop - Pointer {:p}. Memory should be considered invalid.",
        ptr.as_ptr()
    );
}

fn assign_string(g: &ScriptGeneric) {
    let src = g.get_arg_address::<String>(0).unwrap();
    let mut dest = g.get_object::<String>().unwrap();

    eprintln!(
        "[String::assign] Copy value from source {:p} to destination {:p}.",
        src.as_ptr(),
        dest.as_ptr()
    );

    // Ensure the source is valid before copying
    if src.is_null() {
        eprintln!("[String::assign] Source pointer is null. Aborting assignment.");
        return;
    }

    // Assign the source value to the destination object
    dest.set(src.as_ref().clone());

    // Log pointers and their values after assignment
    eprintln!(
        "[String::assign] Final values - Source Pointer {:p}: value = {:?}, Destination Pointer {:p}: value = {:?}.",
        src.as_ptr(),
        src.as_ref(),
        dest.as_ptr(),
        dest.as_ref()
    );

    g.set_return_object(&mut dest.as_void_ptr())
        .expect("Failed to return string");
}

fn add_assign_string(g: &ScriptGeneric) {
    let src = g.get_arg_address::<String>(0).unwrap();
    let mut dest = g.get_object::<String>().unwrap();

    eprintln!(
        "[String::add_assign_string] Copy value from source {:p} to destination {:p}.",
        src.as_ptr(),
        dest.as_ptr()
    );

    // Ensure the source is valid before copying
    if src.is_null() {
        eprintln!("[String::add_assign_string] Source pointer is null. Aborting assignment.");
        return;
    }

    // Add Assign the source value to the destination object
    dest.as_ref_mut().push_str(src.as_ref());

    // Log pointers and their values after assignment
    eprintln!(
        "[String::add_assign_string] Final values - Source Pointer {:p}: value = {:?}, Destination Pointer {:p}: value = {:?}.",
        src.as_ptr(),
        src.as_ref(),
        dest.as_ptr(),
        dest.as_ref()
    );

    g.set_return_object(&mut dest.as_void_ptr())
        .expect("Failed to return string");
}

fn string_equals(g: &ScriptGeneric) {
    let lhs = g.get_object::<String>().unwrap();
    let rhs = g.get_arg_address::<String>(0).unwrap();
    let equal = lhs.as_ref() == rhs.as_ref();
    g.set_return_byte(equal.into()).unwrap();
}

fn string_cmp(g: &ScriptGeneric) {
    let lhs = g.get_object::<String>().unwrap();
    let rhs = g.get_arg_address::<String>(0).unwrap();
    g.set_return_dword(if lhs.as_ref() < rhs.as_ref() {
        -1
    } else if lhs.as_ref() > rhs.as_ref() {
        1
    } else {
        0
    } as u32)
        .unwrap();
}

fn string_add(g: &ScriptGeneric) {
    let lhs = g.get_object::<String>().unwrap();
    let rhs = g.get_arg_address::<String>(0).unwrap();
    let mut ret = g.get_address_of_return_location::<String>().unwrap();
    ret.set(lhs.as_ref().clone() + rhs.as_ref());
}

fn string_length(g: &ScriptGeneric) {
    let obj = g.get_object::<String>().unwrap();
    g.set_return_dword(obj.as_ref().len() as u32).unwrap();
}

fn string_is_empty(g: &ScriptGeneric) {
    let obj = g.get_object::<String>().unwrap();
    g.set_return_byte(obj.as_ref().is_empty().into()).unwrap();
}

fn string_char_at(g: &ScriptGeneric) {
    let idx = g.get_arg_dword(0) as usize;
    let mut obj = g.get_object::<String>().unwrap();

    let str = obj.as_ref_mut();
    if idx >= str.len() {
        let ctx = AngelScript::get_active_context().unwrap();
        ctx.set_exception("Index out of bounds", true).unwrap();
        g.set_return_address_raw(VoidPtr::null()).unwrap();
        return;
    }

    unsafe {
        g.set_return_address(str.as_bytes_mut().get_mut(idx).unwrap())
            .unwrap();
    }
}

// Additional assign/add for primitive types--convert to str and use Ustr.
fn string_assign_int(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<asINT64> = g.get_address_of_arg::<asINT64>(0).unwrap();
    self_ptr.set(value.as_ref().to_string());
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

fn string_assign_uint(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<String>().unwrap();
    let value = g.get_address_of_arg::<asQWORD>(0).unwrap();
    self_ptr.set(value.as_ref().to_string());
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

fn string_assign_double(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<f64> = g.get_address_of_arg::<f64>(0).unwrap();
    self_ptr.set(value.as_ref().to_string());
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

fn string_assign_float(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<f32> = g.get_address_of_arg::<f32>(0).unwrap();
    self_ptr.set(value.as_ref().to_string());
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

fn string_assign_bool(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<bool> = g.get_address_of_arg::<bool>(0).unwrap();
    self_ptr.set(if *value.as_ref() { "true" } else { "false" }.to_string());
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

// Add-assign for primitive types (converts to Ustr and concatenates)
fn string_add_assign_double(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<f64> = g.get_address_of_arg::<f64>(0).unwrap();
    self_ptr.as_ref_mut().push_str(&value.as_ref().to_string());
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

fn string_add_assign_float(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<f32> = g.get_address_of_arg::<f32>(0).unwrap();
    self_ptr.as_ref_mut().push_str(&value.as_ref().to_string());
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

fn string_add_assign_int(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<asINT64> = g.get_address_of_arg::<asINT64>(0).unwrap();
    self_ptr.as_ref_mut().push_str(&value.as_ref().to_string());
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

fn string_add_assign_uint(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<asQWORD> = g.get_address_of_arg::<asQWORD>(0).unwrap();
    self_ptr.as_ref_mut().push_str(&value.as_ref().to_string());
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

fn string_add_assign_bool(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<bool> = g.get_address_of_arg::<bool>(0).unwrap();
    let formatted = if *value.as_ref() { "true" } else { "false" }.to_string();
    self_ptr.as_ref_mut().push_str(&formatted);
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

fn string_add_double(g: &ScriptGeneric) {
    let self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<f64> = g.get_address_of_arg::<f64>(0).unwrap();
    let result = format!("{}{}", self_ptr.as_ref(), value.as_ref());
    g.set_return_object(&mut VoidPtr::from_const_raw(
        Box::into_raw(Box::new(result)) as *mut c_void,
    ));
}

fn double_add_string(g: &ScriptGeneric) {
    let self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<f64> = g.get_address_of_arg::<f64>(0).unwrap();
    let result = format!("{}{}", value.as_ref(), self_ptr.as_ref());
    g.set_return_object(&mut VoidPtr::from_const_raw(
        Box::into_raw(Box::new(result)) as *mut c_void,
    ));
}

fn string_add_float(g: &ScriptGeneric) {
    let self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<f32> = g.get_address_of_arg::<f32>(0).unwrap();
    let result = format!("{}{}", self_ptr.as_ref(), value.as_ref());
    g.set_return_object(&mut VoidPtr::from_const_raw(
        Box::into_raw(Box::new(result)) as *mut c_void,
    ));
}

fn float_add_string(g: &ScriptGeneric) {
    let self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<f32> = g.get_address_of_arg::<f32>(0).unwrap();
    let result = format!("{}{}", value.as_ref(), self_ptr.as_ref());
    g.set_return_object(&mut VoidPtr::from_const_raw(
        Box::into_raw(Box::new(result)) as *mut c_void,
    ));
}

fn string_add_int(g: &ScriptGeneric) {
    let self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<asINT64> = g.get_address_of_arg::<asINT64>(0).unwrap();
    let result = format!("{}{}", self_ptr.as_ref(), value.as_ref());
    g.set_return_object(&mut VoidPtr::from_const_raw(
        Box::into_raw(Box::new(result)) as *mut c_void,
    ));
}

fn int_add_string(g: &ScriptGeneric) {
    let self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<asINT64> = g.get_address_of_arg::<asINT64>(0).unwrap();
    let result = format!("{}{}", value.as_ref(), self_ptr.as_ref());
    g.set_return_object(&mut VoidPtr::from_const_raw(
        Box::into_raw(Box::new(result)) as *mut c_void,
    ));
}

fn string_add_uint(g: &ScriptGeneric) {
    let self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<asQWORD> = g.get_address_of_arg::<asQWORD>(0).unwrap();
    let result = format!("{}{}", self_ptr.as_ref(), value.as_ref());
    g.set_return_object(&mut VoidPtr::from_const_raw(
        Box::into_raw(Box::new(result)) as *mut c_void,
    ));
}

fn uint_add_string(g: &ScriptGeneric) {
    let self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<asQWORD> = g.get_address_of_arg::<asQWORD>(0).unwrap();
    let result = format!("{}{}", value.as_ref(), self_ptr.as_ref());
    g.set_return_object(&mut VoidPtr::from_const_raw(
        Box::into_raw(Box::new(result)) as *mut c_void,
    ));
}

fn string_add_bool(g: &ScriptGeneric) {
    let self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<bool> = g.get_address_of_arg::<bool>(0).unwrap();
    let formatted = if *value.as_ref() { "true" } else { "false" };
    let result = format!("{}{}", self_ptr.as_ref(), formatted);
    g.set_return_object(&mut VoidPtr::from_const_raw(
        Box::into_raw(Box::new(result)) as *mut c_void,
    ));
}

fn bool_add_string(g: &ScriptGeneric) {
    let self_ptr = g.get_object::<String>().unwrap();
    let value: Ptr<bool> = g.get_address_of_arg::<bool>(0).unwrap();
    let formatted = if *value.as_ref() { "true" } else { "false" };
    let result = format!("{}{}", formatted, self_ptr.as_ref());
    let mut ptr = VoidPtr::from_const_raw(Box::into_raw(Box::new(result)) as *mut c_void);
    g.set_return_object(&mut ptr);
}

// Substring
fn string_substring(g: &ScriptGeneric) {
    let self_ptr = g.get_object::<String>().unwrap();
    let start_ptr: Ptr<asUINT> = g.get_address_of_arg::<asUINT>(0).unwrap();
    let len_ptr: Ptr<i32> = g.get_address_of_arg::<i32>(1).unwrap();

    let start = *start_ptr.as_ref() as usize;
    let len = *len_ptr.as_ref() as usize;
    let substring = self_ptr
        .as_ref()
        .get(start..start + len)
        .unwrap_or("")
        .to_string();

    g.get_address_of_return_location::<String>()
        .unwrap()
        .set(substring);
}

// FFI assumed available (replace with real prototype as needed):
// extern "C" fn asScriptGeneric_GetAddressOfReturnLocation(g: *mut asIScriptGeneric) -> *mut std::ffi::c_void;

pub fn with_string_module(engine: &Engine) -> crate::error::Result<()> {
    engine.register_object_type(
        "string",
        size_of::<String>(),
        ObjectTypeFlags::VALUE | ObjectTypeFlags::APP_CLASS_CDAK,
    )?;

    // Register string factory
    engine.register_string_factory(
        "string",
        get_string_factory_instance() as *const _ as *mut asIStringFactory,
    )?;

    engine.register_object_behaviour::<c_void>(
        "string",
        Behaviour::Construct,
        "void f()",
        construct_string,
        None,
        None,
        None,
    )?;

    engine.register_object_behaviour::<c_void>(
        "string",
        Behaviour::Construct,
        "void f(const string &in)",
        copy_construct_string,
        None,
        None,
        None,
    )?;

    engine.register_object_behaviour::<c_void>(
        "string",
        Behaviour::Destruct,
        "void f()",
        destruct_string,
        None,
        None,
        None,
    )?;

    engine.register_object_method::<c_void>(
        "string",
        "string &opAssign(const string &in)",
        assign_string,
        None,
        None,
        None,
    )?;

    engine.register_object_method::<c_void>(
        "string",
        "string &opAddAssign(const string &in)",
        add_assign_string,
        None,
        None,
        None,
    )?;

    engine.register_object_method::<c_void>(
        "string",
        "bool opEquals(const string &in) const",
        string_equals,
        None,
        None,
        None,
    )?;

    engine.register_object_method::<c_void>(
        "string",
        "int opCmp(const string &in) const",
        string_cmp,
        None,
        None,
        None,
    )?;

    // engine.register_object_method(
    //     "string",
    //     "string opAdd(const string &in) const",
    //     string_add,
    //     asCALL_GENERIC,
    // )?;

    // String methods. Add conditional methods as needed; here, the default.
    engine.register_object_method::<c_void>(
        "string",
        "uint length() const",
        string_length,
        None,
        None,
        None,
    )?;

    engine.register_object_method::<c_void>(
        "string",
        "bool isEmpty() const",
        string_is_empty,
        None,
        None,
        None,
    )?;

    // Indexing (mutator & inspector)
    engine.register_object_method::<c_void>(
        "string",
        "uint8 &opIndex(uint)",
        string_char_at,
        None,
        None,
        None,
    )?;

    engine.register_object_method::<c_void>(
        "string",
        "uint8 &opIndex(uint) const",
        string_char_at,
        None,
        None,
        None,
    )?;

    // engine.register_object_method(
    //     "string",
    //     "string &opAssign(double)",
    //     string_assign_double,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string &opAddAssign(double)",
    //     string_add_assign_double,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string opAdd(double) const",
    //     string_add_double,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string opAdd_r(double) const",
    //     double_add_string,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string &opAssign(float)",
    //     string_assign_float,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string &opAddAssign(float)",
    //     string_add_assign_float,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string opAdd(float) const",
    //     string_add_float,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string opAdd_r(float) const",
    //     float_add_string,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string &opAssign(int64)",
    //     string_assign_int,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string &opAddAssign(int64)",
    //     string_add_assign_int,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string opAdd(int64) const",
    //     string_add_int,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string opAdd_r(int64) const",
    //     int_add_string,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string &opAssign(uint64)",
    //     string_assign_uint,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string &opAddAssign(uint64)",
    //     string_add_assign_uint,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string opAdd(uint64) const",
    //     string_add_uint,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string opAdd_r(uint64) const",
    //     uint_add_string,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string &opAssign(bool)",
    //     string_assign_bool,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string &opAddAssign(bool)",
    //     string_add_assign_bool,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string opAdd(bool) const",
    //     string_add_bool,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string opAdd_r(bool) const",
    //     bool_add_string,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method(
    //     "string",
    //     "string substr(uint start = 0, int count = -1) const",
    //     string_substring,
    //     asCALL_GENERIC,
    // )?;
    // engine.register_object_method("string", "int findFirst(const string &in, uint start = 0) const", string_find_first, asCALL_GENERIC)?;
    // engine.register_object_method("string", "int findFirstOf(const string &in, uint start = 0) const", string_find_first_of, asCALL_GENERIC)?;
    // engine.register_object_method("string", "int findFirstNotOf(const string &in, uint start = 0) const", string_find_first_not_of, asCALL_GENERIC)?;
    // engine.register_object_method("string", "int findLast(const string &in, int start = -1) const", string_find_last, asCALL_GENERIC)?;
    // engine.register_object_method("string", "int findLastOf(const string &in, int start = -1) const", string_find_last_of, asCALL_GENERIC)?;
    // engine.register_object_method("string", "int findLastNotOf(const string &in, int start = -1) const", string_find_last_not_of, asCALL_GENERIC)?;
    // engine.register_object_method("string", "void insert(uint pos, const string &in other)", string_insert, asCALL_GENERIC)?;
    // engine.register_object_method("string", "void erase(uint pos, int count = -1)", string_erase, asCALL_GENERIC)?;
    //
    // engine.register_global_function("string formatInt(int64 val, const string &in options = \"\", uint width = 0)", format_int, asCALL_GENERIC)?;
    // engine.register_global_function("string formatUInt(uint64 val, const string &in options = \"\", uint width = 0)", format_uint, asCALL_GENERIC)?;
    // engine.register_global_function("string formatFloat(double val, const string &in options = \"\", uint width = 0, uint precision = 0)", format_float, asCALL_GENERIC)?;
    // engine.register_global_function("int64 parseInt(const string &in, uint base = 10, uint &out byteCount = 0)", parse_int, asCALL_GENERIC)?;
    // engine.register_global_function("uint64 parseUInt(const string &in, uint base = 10, uint &out byteCount = 0)", parse_u_int, asCALL_GENERIC)?;
    // engine.register_global_function("double parseFloat(const string &in, uint &out byteCount = 0)", parse_float, asCALL_GENERIC)?;

    Ok(())
}
