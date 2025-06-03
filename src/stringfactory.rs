use crate::{Behaviours, Engine, Error, ObjTypeFlags, Ptr, ScriptGeneric, VoidPtr};
use angelscript_bindings::asECallConvTypes::asCALL_GENERIC;
use angelscript_bindings::{
    asContext_SetException, asEngine_RegisterStringFactory, asGetActiveContext, asINT32, asINT64,
    asIStringFactory, asIStringFactory__bindgen_vtable, asQWORD, asUINT,
};
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CString};
use std::sync::{Arc, Mutex, OnceLock};
use ustr::{Ustr, UstrMap};

struct InnerCache {
    // From Ustr to pointer handle AngelScript uses
    ustr_to_ptr: UstrMap<VoidPtr>,
    // From pointer to Ustr, to retrieve string from the handle
    ptr_to_ustr: HashMap<VoidPtr, Ustr>,
}

impl InnerCache {
    fn new() -> Self {
        Self {
            ustr_to_ptr: UstrMap::default(),
            ptr_to_ustr: HashMap::new(),
        }
    }

    fn get_or_intern(&mut self, bytes: &[u8]) -> (VoidPtr, Ustr) {
        // Only at FFI boundary: decode from &[u8] to Ustr directly
        let us = match std::str::from_utf8(bytes) {
            Ok(s) => Ustr::from(s),
            Err(_) => Ustr::from("�"), // fallback for invalid utf-8
        };
        if let Some(&ptr) = self.ustr_to_ptr.get(&us) {
            (ptr, us)
        } else {
            let boxed = Box::into_raw(Box::new(us.clone())) as *const c_void;
            let ptr = VoidPtr::from_const_raw(boxed);
            self.ustr_to_ptr.insert(us, ptr);
            self.ptr_to_ustr.insert(ptr, us.clone());
            (ptr, us)
        }
    }

    fn remove(&mut self, ptr: VoidPtr) -> bool {
        if let Some(us) = self.ptr_to_ustr.remove(&ptr) {
            self.ustr_to_ptr.remove(&us);
            unsafe {
                let _ = Box::from_raw(ptr.as_ptr() as *mut Ustr);
            }
            true
        } else {
            false
        }
    }

    fn get_ustr(&self, ptr: VoidPtr) -> Option<Ustr> {
        self.ptr_to_ustr.get(&ptr).cloned()
    }
}

#[derive(Clone)]
pub struct StringFactory {
    cache: Arc<Mutex<InnerCache>>,
}

impl StringFactory {
    pub fn singleton() -> &'static StringFactory {
        static INSTANCE: OnceLock<StringFactory> = OnceLock::new();
        INSTANCE.get_or_init(|| StringFactory {
            cache: Arc::new(Mutex::new(InnerCache::new())),
        })
    }

    pub unsafe extern "C" fn get_string_constant(
        _this: *mut asIStringFactory,
        data: *const c_char,
        length: asUINT,
    ) -> *const c_void {
        // FFI boundary: Only use &str for safe decode
        let sf = StringFactory::singleton();
        let slice = unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };
        let mut cache = sf.cache.lock().unwrap();
        let (ptr, _) = cache.get_or_intern(slice);
        ptr.as_ptr()
    }

    pub unsafe extern "C" fn release_string_constant(
        _this: *mut asIStringFactory,
        str_: *const c_void,
    ) -> c_int {
        let sf = StringFactory::singleton();
        let mut cache = sf.cache.lock().unwrap();
        let ptr = VoidPtr::from_const_raw(str_);
        if cache.remove(ptr) { 1 } else { 0 }
    }

    pub unsafe extern "C" fn get_raw_string_data(
        _this: *const asIStringFactory,
        str_: *const c_void,
        data: *mut c_char,
        length: *mut asUINT,
    ) -> c_int {
        let sf = StringFactory::singleton();
        let cache = sf.cache.lock().unwrap();
        let ptr = VoidPtr::from_const_raw(str_);
        if let Some(us) = cache.get_ustr(ptr) {
            let bytes = us.as_bytes();
            if !data.is_null() {
                unsafe {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), data as *mut u8, bytes.len())
                };
            }
            if !length.is_null() {
                unsafe { *length = bytes.len() as asUINT };
            }
            1
        } else {
            0
        }
    }

    fn assert_cache_empty(&self) {
        let cache = self.cache.lock().unwrap();
        assert!(cache.ptr_to_ustr.is_empty());
        assert!(cache.ustr_to_ptr.is_empty());
    }
}

impl Drop for StringFactory {
    fn drop(&mut self) {
        self.assert_cache_empty();
    }
}

unsafe impl Send for StringFactory {}
unsafe impl Sync for StringFactory {}

// VTable instance must remain alive
static STRING_FACTORY_VTABLE: asIStringFactory__bindgen_vtable = asIStringFactory__bindgen_vtable {
    asIStringFactory_GetStringConstant: StringFactory::get_string_constant,
    asIStringFactory_ReleaseStringConstant: StringFactory::release_string_constant,
    asIStringFactory_GetRawStringData: StringFactory::get_raw_string_data,
};

pub fn get_string_factory_instance() -> &'static asIStringFactory {
    static INSTANCE: OnceLock<asIStringFactory> = OnceLock::new();
    INSTANCE.get_or_init(|| asIStringFactory {
        vtable_: &STRING_FACTORY_VTABLE,
    })
}

fn generic_error(msg: &str) {
    unsafe {
        let ctx = asGetActiveContext();
        asContext_SetException(ctx, Ustr::from(msg).as_char_ptr());
    };
}

// Constructor from &str (used from AngelScript generics)
fn construct_string(g: &ScriptGeneric) {
    let ustr = Ustr::from("");
    g.get_object::<Ustr>().set(ustr);
}

// Copy constructor for Ustr "string"
fn copy_construct_string(g: &ScriptGeneric) {
    let src_ptr = g.get_arg_object::<Ustr>(0);
    let new_ustr = Ustr::from(src_ptr.as_ref());
    g.get_object::<Ustr>().set(new_ustr);
}

// Destructor: free the storage.
fn destruct_string(g: &ScriptGeneric) {
    let ptr = g.get_object::<Ustr>().as_mut_ptr();
    if !ptr.is_null() {
        unsafe {
            // Run the destructor in place; does nothing for Ustr,
            // but is correct for idiomatic Rust memory management
            std::ptr::drop_in_place(ptr);
        }
    }
}

// Assignment from another string.
fn assign_string(g: &ScriptGeneric) {
    let mut this = g.get_object::<Ustr>();
    let mut src_ptr = g.get_arg_object::<Ustr>(0);
    unsafe {
        *this.as_mut_ptr() = *src_ptr.as_mut_ptr();
    }
    g.set_return_address(&mut this.as_void_ptr());
}

// Add/assign (+=) - concatenation, using Ustr.
fn add_assign_string(g: &ScriptGeneric) {
    let mut this = g.get_object::<Ustr>();
    let a = g.get_arg_object::<Ustr>(0);
    let this_str = this.as_ref().as_str();
    let a_str = a.as_ref().as_str();
    let mut s = String::with_capacity(this_str.len() + a_str.len());
    s.push_str(this_str);
    s.push_str(a_str);
    let joined = Ustr::from(&s);
    unsafe {
        this.set(joined);
    }
    g.set_return_address(&mut this.as_void_ptr());
}

// String equality.
fn string_equals(g: &ScriptGeneric) {
    let lhs_ptr = g.get_object::<Ustr>();
    let rhs_ptr = g.get_arg_address::<Ustr>(0);
    let equal = lhs_ptr.as_ref().eq(rhs_ptr.as_ref());
    g.get_address_of_return_location::<bool>().set(equal);
}

// Compare two strings (-1/0/1).
fn string_cmp(g: &ScriptGeneric) {
    let lhs_ptr = g.get_object::<Ustr>();
    let rhs_ptr = g.get_arg_address::<Ustr>(0);
    let ordering = lhs_ptr.as_ref().cmp(rhs_ptr.as_ref());
    let result = match ordering {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    };
    g.get_address_of_return_location::<asINT32>()
        .set(result as asINT32);
}

// String addition (returns new Ustr*).
fn string_add(g: &ScriptGeneric) {
    let lhs_ptr = g.get_object::<Ustr>();
    let rhs_ptr = g.get_arg_address::<Ustr>(0);
    // Assuming lhs_ptr and rhs_ptr are *const Ustr and valid.
    let lhs = lhs_ptr.as_ref().as_str();
    let rhs = rhs_ptr.as_ref().as_str();

    // Preallocate the String with exact size and append manually.
    let mut buf = String::with_capacity(lhs.len() + rhs.len());
    buf.push_str(lhs);
    buf.push_str(rhs);

    // Now intern.
    let new_ustr = Ustr::from(&buf);
    g.set_return_object(&mut new_ustr.as_char_ptr().into());
}

// String length
fn string_length(g: &ScriptGeneric) {
    let self_ptr = g.get_object::<Ustr>();
    let len = unsafe { self_ptr.as_ref().len() };
    g.get_address_of_return_location::<asUINT>()
        .set(len as asUINT);
}

// String is_empty
fn string_is_empty(g: &ScriptGeneric) {
    let self_ptr = g.get_object::<Ustr>();
    g.get_address_of_return_location::<bool>()
        .set(self_ptr.as_ref().is_empty());
}

// Get character at index
fn string_char_at(g: &ScriptGeneric) {
    let self_ptr = g.get_object::<Ustr>();
    let idx = g.get_arg_dword(0);

    if let Some(ch) = self_ptr.as_ref().chars().nth(idx as usize) {
        g.get_address_of_return_location::<u8>().set(ch as u8);
    } else {
        generic_error("String index out of range");
        g.get_address_of_return_location().set(0);
    }
}

// Additional assign/add for primitive types--convert to str and use Ustr.
fn string_assign_int(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<asINT64> = g.get_address_of_arg::<asINT64>(0);
    self_ptr.set(Ustr::from(itoa::Buffer::new().format(*value.as_ref())));
    g.set_return_address(&mut self_ptr.as_void_ptr())
}
fn string_assign_uint(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<asQWORD> = g.get_address_of_arg::<asQWORD>(0);
    self_ptr.set(Ustr::from(itoa::Buffer::new().format(*value.as_ref())));
    g.set_return_address(&mut self_ptr.as_void_ptr())
}
fn string_assign_double(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<f64> = g.get_address_of_arg::<f64>(0);
    self_ptr.set(Ustr::from(ryu::Buffer::new().format(*value.as_ref())));
    g.set_return_address(&mut self_ptr.as_void_ptr())
}
fn string_assign_float(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<f32> = g.get_address_of_arg::<f32>(0);
    self_ptr.set(Ustr::from(ryu::Buffer::new().format(*value.as_ref())));
    g.set_return_address(&mut self_ptr.as_void_ptr())
}
fn string_assign_bool(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<bool> = g.get_address_of_arg::<bool>(0);
    let s = if *value.as_ref() {
        Ustr::from("true")
    } else {
        Ustr::from("false")
    };
    self_ptr.set(s);
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

// Add-assign for primitive types (converts to Ustr and concatenates)
fn string_add_assign_double(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<f64> = g.get_address_of_arg::<f64>(0);
    let orig = self_ptr.as_ref().as_str();
    let mut buf = ryu::Buffer::new();
    let suffix = buf.format_finite(*value.as_ref());

    let mut s = String::with_capacity(orig.len() + suffix.len());
    s.push_str(orig);
    s.push_str(suffix);

    self_ptr.set(Ustr::from(s.as_str()));
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

fn string_add_assign_float(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<f32> = g.get_address_of_arg::<f32>(0);
    let orig = self_ptr.as_ref().as_str();
    let mut buf = ryu::Buffer::new();
    let suffix = buf.format_finite(*value.as_ref());

    let mut s = String::with_capacity(orig.len() + suffix.len());
    s.push_str(orig);
    s.push_str(suffix);

    self_ptr.set(Ustr::from(s.as_str()));
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

fn string_add_assign_int(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<asINT64> = g.get_address_of_arg::<asINT64>(0);
    let orig = self_ptr.as_ref().as_str();
    let mut buf = itoa::Buffer::new();
    let suffix = buf.format(*value.as_ref());

    let mut s = String::with_capacity(orig.len() + suffix.len());
    s.push_str(orig);
    s.push_str(suffix);

    self_ptr.set(Ustr::from(s.as_str()));
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

fn string_add_assign_uint(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<asQWORD> = g.get_address_of_arg::<asQWORD>(0);
    let orig = self_ptr.as_ref().as_str();
    let mut buf = itoa::Buffer::new();
    let suffix = buf.format(*value.as_ref());

    let mut s = String::with_capacity(orig.len() + suffix.len());
    s.push_str(orig);
    s.push_str(suffix);

    self_ptr.set(Ustr::from(s.as_str()));
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

fn string_add_assign_bool(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<bool> = g.get_address_of_arg::<bool>(0);
    let orig = self_ptr.as_ref().as_str();
    let suffix = if *value.as_ref() {
        "true"
    } else {
        "false"
    };

    let mut s = String::with_capacity(orig.len() + suffix.len());
    s.push_str(orig);
    s.push_str(suffix);

    self_ptr.set(Ustr::from(s.as_str()));
    g.set_return_address(&mut self_ptr.as_void_ptr());
}

fn string_add_double(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<f64> = g.get_address_of_arg::<f64>(0);
    let orig = self_ptr.as_ref().as_str();
    let mut buf = ryu::Buffer::new();
    let suffix = buf.format_finite(*value.as_ref());

    let mut s = String::with_capacity(orig.len() + suffix.len());
    s.push_str(orig);
    s.push_str(suffix);

    let new_ustr = Ustr::from(s.as_str());
    g.set_return_object(&mut new_ustr.as_char_ptr().into());
}

fn double_add_string(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<f64> = g.get_address_of_arg::<f64>(0);
    let orig = self_ptr.as_ref().as_str();
    let mut buf = ryu::Buffer::new();
    let prefix = buf.format_finite(*value.as_ref());

    let mut s = String::with_capacity(orig.len() + prefix.len());
    s.push_str(prefix);
    s.push_str(orig);

    let new_ustr = Ustr::from(s.as_str());
    g.set_return_object(&mut new_ustr.as_char_ptr().into());
}

fn string_add_float(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<f32> = g.get_address_of_arg::<f32>(0);
    let orig = self_ptr.as_ref().as_str();
    let mut buf = ryu::Buffer::new();
    let suffix = buf.format_finite(*value.as_ref());

    let mut s = String::with_capacity(orig.len() + suffix.len());
    s.push_str(orig);
    s.push_str(suffix);

    let new_ustr = Ustr::from(s.as_str());
    g.set_return_object(&mut new_ustr.as_char_ptr().into());
}

fn float_add_string(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<f32> = g.get_address_of_arg::<f32>(0);
    let orig = self_ptr.as_ref().as_str();
    let mut buf = ryu::Buffer::new();
    let prefix = buf.format_finite(*value.as_ref());

    let mut s = String::with_capacity(orig.len() + prefix.len());
    s.push_str(prefix);
    s.push_str(orig);

    let new_ustr = Ustr::from(s.as_str());
    g.set_return_object(&mut new_ustr.as_char_ptr().into());
}

fn string_add_int(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<asINT64> = g.get_address_of_arg::<asINT64>(0);
    let orig = self_ptr.as_ref().as_str();
    let mut buf = itoa::Buffer::new();
    let suffix = buf.format(*value.as_ref());

    let mut s = String::with_capacity(orig.len() + suffix.len());
    s.push_str(orig);
    s.push_str(suffix);

    let new_ustr = Ustr::from(s.as_str());
    g.set_return_object(&mut new_ustr.as_char_ptr().into());
}

fn int_add_string(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<asINT64> = g.get_address_of_arg::<asINT64>(0);
    let orig = self_ptr.as_ref().as_str();
    let mut buf = itoa::Buffer::new();
    let prefix = buf.format(*value.as_ref());

    let mut s = String::with_capacity(orig.len() + prefix.len());
    s.push_str(prefix);
    s.push_str(orig);

    let new_ustr = Ustr::from(s.as_str());
    g.set_return_object(&mut new_ustr.as_char_ptr().into());
}

fn string_add_uint(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<asQWORD> = g.get_address_of_arg::<asQWORD>(0);
    let orig = self_ptr.as_ref().as_str();
    let mut buf = itoa::Buffer::new();
    let suffix = buf.format(*value.as_ref());

    let mut s = String::with_capacity(orig.len() + suffix.len());
    s.push_str(orig);
    s.push_str(suffix);

    let new_ustr = Ustr::from(s.as_str());
    g.set_return_object(&mut new_ustr.as_char_ptr().into());
}

fn uint_add_string(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<asQWORD> = g.get_address_of_arg::<asQWORD>(0);
    let orig = self_ptr.as_ref().as_str();
    let mut buf = itoa::Buffer::new();
    let prefix = buf.format(*value.as_ref());

    let mut s = String::with_capacity(orig.len() + prefix.len());
    s.push_str(prefix);
    s.push_str(orig);

    let new_ustr = Ustr::from(s.as_str());
    g.set_return_object(&mut new_ustr.as_char_ptr().into());
}

fn string_add_bool(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<bool> = g.get_address_of_arg::<bool>(0);
    let orig = self_ptr.as_ref().as_str();
    let suffix = if *value.as_ref() {
        "true"
    } else {
        "false"
    };

    let mut s = String::with_capacity(orig.len() + suffix.len());
    s.push_str(orig);
    s.push_str(suffix);

    let new_ustr = Ustr::from(s.as_str());
    g.set_return_object(&mut new_ustr.as_char_ptr().into());
}

fn bool_add_string(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let value: Ptr<bool> = g.get_address_of_arg::<bool>(0);
    let orig = self_ptr.as_ref().as_str();
    let prefix = if *value.as_ref() {
        "true"
    } else {
        "false"
    };

    let mut s = String::with_capacity(orig.len() + prefix.len());
    s.push_str(prefix);
    s.push_str(orig);

    let new_ustr = Ustr::from(s.as_str());
    g.set_return_object(&mut new_ustr.as_char_ptr().into());
}

// Substring
fn string_substring(g: &ScriptGeneric) {
    let mut self_ptr = g.get_object::<Ustr>();
    let start_ptr: Ptr<asUINT> = g.get_address_of_arg::<asUINT>(0);
    let len_ptr: Ptr<i32> = g.get_address_of_arg::<i32>(1);
    let start = *start_ptr.as_ref() as usize;
    let len = *len_ptr.as_ref() as usize;
    let substr = self_ptr.as_ref().get(start..start + len).unwrap_or("");
    let ret_ustr = Ustr::from(substr);
    g.get_address_of_return_location::<Ustr>().set(ret_ustr)
}

// FFI assumed available (replace with real prototype as needed):
// extern "C" fn asScriptGeneric_GetAddressOfReturnLocation(g: *mut asIScriptGeneric) -> *mut std::ffi::c_void;

pub unsafe fn register_cstring(engine: &Engine) -> crate::error::Result<()> {
    engine.register_object_type(
        "string",
        size_of::<*const c_char>(),
        vec![
            ObjTypeFlags::asOBJ_VALUE,
            ObjTypeFlags::asOBJ_APP_CLASS_CDAK,
        ],
    )?;

    // Register string factory
    let r = unsafe {
        let str = CString::new("string").unwrap();
        asEngine_RegisterStringFactory(
            engine.as_ptr(),
            str.as_ptr(),
            get_string_factory_instance() as *const _ as *mut asIStringFactory,
        )
    };
    if r < 0 {
        return Error::from_code(r);
    }

    engine.register_object_behaviour(
        "string",
        Behaviours::asBEHAVE_CONSTRUCT,
        "void f()",
        construct_string,
        asCALL_GENERIC,
    )?;

    engine.register_object_behaviour(
        "string",
        Behaviours::asBEHAVE_CONSTRUCT,
        "void f(const string &in)",
        copy_construct_string,
        asCALL_GENERIC,
    )?;

    engine.register_object_behaviour(
        "string",
        Behaviours::asBEHAVE_DESTRUCT,
        "void f()",
        destruct_string,
        asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "string &opAssign(const string &in)",
        assign_string,
        asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "string &opAddAssign(const string &in)",
        add_assign_string,
        asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "bool opEquals(const string &in) const",
        string_equals,
        asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "int opCmp(const string &in) const",
        string_cmp,
        asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "string opAdd(const string &in) const",
        string_add,
        asCALL_GENERIC,
    )?;

    // String methods. Add conditional methods as needed; here, the default.
    engine.register_object_method(
        "string",
        "uint length() const",
        string_length,
        asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "bool isEmpty() const",
        string_is_empty,
        asCALL_GENERIC,
    )?;

    // Indexing (mutator & inspector)
    engine.register_object_method(
        "string",
        "uint8 &opIndex(uint)",
        string_char_at,
        asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "uint8 &opIndex(uint) const",
        string_char_at,
        asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "string &opAssign(double)",
        string_assign_double,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string &opAddAssign(double)",
        string_add_assign_double,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string opAdd(double) const",
        string_add_double,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string opAdd_r(double) const",
        double_add_string,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string &opAssign(float)",
        string_assign_float,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string &opAddAssign(float)",
        string_add_assign_float,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string opAdd(float) const",
        string_add_float,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string opAdd_r(float) const",
        float_add_string,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string &opAssign(int64)",
        string_assign_int,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string &opAddAssign(int64)",
        string_add_assign_int,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string opAdd(int64) const",
        string_add_int,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string opAdd_r(int64) const",
        int_add_string,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string &opAssign(uint64)",
        string_assign_uint,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string &opAddAssign(uint64)",
        string_add_assign_uint,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string opAdd(uint64) const",
        string_add_uint,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string opAdd_r(uint64) const",
        uint_add_string,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string &opAssign(bool)",
        string_assign_bool,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string &opAddAssign(bool)",
        string_add_assign_bool,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string opAdd(bool) const",
        string_add_bool,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string opAdd_r(bool) const",
        bool_add_string,
        asCALL_GENERIC,
    )?;
    engine.register_object_method(
        "string",
        "string substr(uint start = 0, int count = -1) const",
        string_substring,
        asCALL_GENERIC,
    )?;
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