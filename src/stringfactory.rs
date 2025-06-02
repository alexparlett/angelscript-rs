use crate::ffi::{
    asContext_SetException, asEngine_RegisterStringFactory, asGetActiveContext,
    asIScriptGeneric_GetAddressOfArg, asIScriptGeneric_GetAddressOfReturnLocation,
    asIScriptGeneric_GetArgAddress, asIScriptGeneric_GetArgDWord, asIScriptGeneric_GetArgObject,
    asIScriptGeneric_GetObject, asIScriptGeneric_SetReturnAddress,
    asIScriptGeneric_SetReturnObject, asIStringFactory, asIStringFactory__bindgen_vtable, asUINT,
};
use crate::{Behaviours, CallConvTypes, Engine, Error, ObjTypeFlags, ScriptGeneric};
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Hash, PartialEq, Eq, Copy, Clone, Debug)]
struct VoidPtr(*const c_void);

unsafe impl Send for VoidPtr {}
unsafe impl Sync for VoidPtr {}

// Cache entry (keeps CString alive and reference count)
#[derive(Debug, Clone)]
struct StringConstant {
    cstring: CString,
    refs: usize,
}

struct InnerCache {
    // Key: pointer to the internal C data, as AngelScript will use the pointer as handle
    map: HashMap<VoidPtr, (StringConstant, String)>,
    // For lookup to avoid duplicate allocations (Rust string to handle)
    str_lookup: HashMap<String, VoidPtr>,
}

impl InnerCache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            str_lookup: HashMap::new(),
        }
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

    // C API: get string constant (returns pointer to string memory)
    pub unsafe extern "C" fn get_string_constant(
        this: *mut asIStringFactory,
        data: *const c_char,
        length: asUINT,
    ) -> *const c_void {
        let factory = StringFactory::singleton();
        let slice = unsafe { std::slice::from_raw_parts(data as *const u8, length as usize) };

        let s = match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return std::ptr::null(), // or handle error
        };

        let mut cache = factory.cache.lock().unwrap();

        // Pull out the pointer to break the borrow
        let ptr_opt = cache.str_lookup.get(s).copied();

        if let Some(ptr) = ptr_opt {
            if let Some((string_constant, _)) = cache.map.get_mut(&ptr) {
                string_constant.refs += 1;
            }
            return ptr.0;
        }

        // Not in cache, allocate new CString
        let cstring = match CString::new(s) {
            Ok(cs) => cs,
            Err(_) => return std::ptr::null(),
        };
        let ptr = VoidPtr(cstring.as_ptr() as *const c_void);
        let cache_entry = StringConstant { cstring, refs: 1 };
        cache.map.insert(ptr, (cache_entry, s.to_owned()));
        cache.str_lookup.insert(s.to_owned(), ptr);
        ptr.0
    }

    // C API: release string constant (decrements refcount, or frees if last)
    pub unsafe extern "C" fn release_string_constant(
        this: *mut asIStringFactory,
        str_: *const c_void,
    ) -> c_int {
        if str_.is_null() {
            return -1; // error
        }
        let factory = StringFactory::singleton();
        let mut cache = factory.cache.lock().unwrap();

        let entry_removed = {
            if let Some((entry, rust_str)) = cache.map.get_mut(&VoidPtr(str_)) {
                if entry.refs == 0 {
                    return -1;
                }
                entry.refs -= 1;
                if entry.refs == 0 {
                    // Copy the key for later removal, and rust_str too if not Copy
                    Some((VoidPtr(str_), rust_str.clone()))
                } else {
                    return 0;
                }
            } else {
                return -1;
            }
        };

        if let Some((ptr, rust_str)) = entry_removed {
            // Now we have no outstanding borrows
            cache.str_lookup.remove(&rust_str);
            cache.map.remove(&ptr);
        }

        0
    }

    // C API: get raw string data
    pub unsafe extern "C" fn get_raw_string_data(
        this: *const asIStringFactory,
        str_: *const c_void,
        data: *mut c_char,
        length: *mut asUINT,
    ) -> c_int {
        if str_.is_null() {
            return -1;
        }
        let factory = StringFactory::singleton();
        let cache = factory.cache.lock().unwrap();
        if let Some((entry, _)) = cache.map.get(&VoidPtr(str_)) {
            let buf = entry.cstring.as_bytes();
            if !length.is_null() {
                unsafe {
                    *length = buf.len() as asUINT;
                }
            }
            if !data.is_null() {
                unsafe {
                    std::ptr::copy_nonoverlapping(buf.as_ptr() as *const c_char, data, buf.len())
                };
            }
            0
        } else {
            -1
        }
    }

    // Clean shutdown/debug: ensures all references released
    pub fn assert_cache_empty(&self) {
        let cache = self.cache.lock().unwrap();
        assert!(
            cache.map.is_empty(),
            "StringConstant cache is not empty on shutdown!"
        );
    }
}

impl Drop for StringFactory {
    fn drop(&mut self) {
        self.assert_cache_empty();
    }
}

// VTable instance (must remain alive)
static STRING_FACTORY_VTABLE: asIStringFactory__bindgen_vtable = asIStringFactory__bindgen_vtable {
    asIStringFactory_GetStringConstant: StringFactory::get_string_constant,
    asIStringFactory_ReleaseStringConstant: StringFactory::release_string_constant,
    asIStringFactory_GetRawStringData: StringFactory::get_raw_string_data,
};

// Factory singleton accessor for FFI
pub fn get_string_factory_instance() -> &'static asIStringFactory {
    static INSTANCE: OnceLock<asIStringFactory> = OnceLock::new();
    INSTANCE.get_or_init(|| asIStringFactory {
        vtable_: &STRING_FACTORY_VTABLE,
    })
}

fn construct_string(g: &ScriptGeneric) {
    use std::ffi::CString;
    use std::os::raw::c_char;

    unsafe {
        // AngelScript will provide where to write the *mut c_char (pointer to buffer)
        let obj_ptr = asIScriptGeneric_GetObject(g.as_ptr()) as *mut *mut c_char;
        if obj_ptr.is_null() {
            return;
        }

        // Allocate a new empty string
        let cstring = CString::new("").unwrap();

        // Convert to raw pointer (leak it, AngelScript is now responsible)
        *obj_ptr = cstring.into_raw();
    }
}

fn copy_construct_string(g: &ScriptGeneric) {
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

    unsafe {
        // 1. Get source string pointer
        let src = asIScriptGeneric_GetArgObject(g.as_ptr(), 0) as *const c_char;
        // 2. Get destination pointer (output pointer-to-pointer)
        let dst = asIScriptGeneric_GetObject(g.as_ptr()) as *mut *mut c_char;

        if !dst.is_null() && !src.is_null() {
            // 3. Clone the content: create a new CString
            let cstr = CStr::from_ptr(src);
            // Allocate new C string (with null termination)
            let new_cstring = CString::new(cstr.to_bytes()).unwrap();
            // Convert CString to raw pointer (leak it for AngelScript to manage)
            *dst = new_cstring.into_raw();
        } else if !dst.is_null() {
            // If basic is null, set output to null
            *dst = std::ptr::null_mut();
        }
    }
}

fn destruct_string(g: &ScriptGeneric) {
    use std::ffi::CString;
    use std::os::raw::c_char;

    unsafe {
        // The object is a pointer to a heap-allocated C string
        let obj_ptr = asIScriptGeneric_GetObject(g.as_ptr()) as *mut *mut c_char;
        if obj_ptr.is_null() || (*obj_ptr).is_null() {
            return;
        }

        // Take ownership and free the memory
        let _ = CString::from_raw(*obj_ptr);

        // Not strictly required, but clears the pointer in case
        *obj_ptr = std::ptr::null_mut();
    }
}

fn assign_string(g: &ScriptGeneric) {
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

    unsafe {
        // Get source string pointer
        let a_ptr = asIScriptGeneric_GetArgObject(g.as_ptr(), 0) as *mut c_char;
        // Get self string pointer location
        let self_ptr = asIScriptGeneric_GetObject(g.as_ptr()) as *mut *mut c_char;

        if self_ptr.is_null() {
            return;
        }

        // Free the old string if it exists
        if !(*self_ptr).is_null() {
            let _ = CString::from_raw(*self_ptr);
        }

        // Duplicate the source string into a new allocation
        let new_cstring = if !a_ptr.is_null() {
            let c_str = CStr::from_ptr(a_ptr);
            Some(CString::new(c_str.to_bytes()).unwrap())
        } else {
            Some(CString::new("").unwrap())
        };

        // Assign new pointer to self
        *self_ptr = new_cstring.unwrap().into_raw();

        // Set return address to self (AngelScript expects this)
        asIScriptGeneric_SetReturnAddress(g.as_ptr(), self_ptr as *mut std::ffi::c_void);
    }
}

fn add_assign_string(g: &ScriptGeneric) {
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

    unsafe {
        // Get pointer to source string
        let a_ptr = asIScriptGeneric_GetArgObject(g.as_ptr(), 0) as *const c_char;
        // Get pointer to self's location (to modify in place)
        let self_ptr = asIScriptGeneric_GetObject(g.as_ptr()) as *mut *mut c_char;

        if self_ptr.is_null() {
            return;
        }

        // Prepare concatenation
        let mut result = String::new();

        // Add self's string if not null
        if !(*self_ptr).is_null() {
            let self_cstr = CStr::from_ptr(*self_ptr);
            result.push_str(self_cstr.to_str().unwrap_or(""));
        }

        // Add the right-hand side if not null
        if !a_ptr.is_null() {
            let a_cstr = CStr::from_ptr(a_ptr);
            result.push_str(a_cstr.to_str().unwrap_or(""));
        }

        // Release old string
        if !(*self_ptr).is_null() {
            let _ = CString::from_raw(*self_ptr);
        }

        // Store new concatenated value as CString
        let new_cstring = CString::new(result).unwrap();
        *self_ptr = new_cstring.into_raw();

        // Set return address to self for AngelScript
        asIScriptGeneric_SetReturnAddress(g.as_ptr(), self_ptr as *mut std::ffi::c_void);
    }
}

fn string_equals(g: &ScriptGeneric) {
    use std::ffi::CStr;
    use std::os::raw::c_char;

    unsafe {
        // Get self string
        let a_ptr = asIScriptGeneric_GetObject(g.as_ptr()) as *const c_char;
        // Get other string (parameter)
        let b_ptr = asIScriptGeneric_GetArgAddress(g.as_ptr(), 0) as *const c_char;

        // CStr::from_ptr expects non-null, so handle null pointers
        let a_str = if !a_ptr.is_null() {
            CStr::from_ptr(a_ptr).to_bytes()
        } else {
            b""
        };

        let b_str = if !b_ptr.is_null() {
            CStr::from_ptr(b_ptr).to_bytes()
        } else {
            b""
        };

        // Do byte comparison (safe for C strings)
        let is_equal = a_str == b_str;

        // Get the location to write the return value (expects a bool)
        let ret_ptr = asIScriptGeneric_GetAddressOfReturnLocation(g.as_ptr()) as *mut bool;
        if !ret_ptr.is_null() {
            *ret_ptr = is_equal;
        }
    }
}

fn string_cmp(g: &ScriptGeneric) {
    use std::ffi::CStr;
    use std::os::raw::c_char;

    unsafe {
        // Get the first string (`self`)
        let a_ptr = asIScriptGeneric_GetObject(g.as_ptr()) as *const c_char;
        // Get the second string (argument)
        let b_ptr = asIScriptGeneric_GetArgAddress(g.as_ptr(), 0) as *const c_char;

        let a_str = if !a_ptr.is_null() {
            CStr::from_ptr(a_ptr).to_bytes()
        } else {
            b""
        };

        let b_str = if !b_ptr.is_null() {
            CStr::from_ptr(b_ptr).to_bytes()
        } else {
            b""
        };

        // Perform comparison
        let cmp = if a_str < b_str {
            -1
        } else if a_str > b_str {
            1
        } else {
            0
        };

        // Write the result to the return location
        let ret_ptr = asIScriptGeneric_GetAddressOfReturnLocation(g.as_ptr()) as *mut i32;
        if !ret_ptr.is_null() {
            *ret_ptr = cmp;
        }
    }
}

fn string_add(g: &ScriptGeneric) {
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

    unsafe {
        // Get first and second string pointers
        let a_ptr = asIScriptGeneric_GetObject(g.as_ptr()) as *const c_char;
        let b_ptr = asIScriptGeneric_GetArgAddress(g.as_ptr(), 0) as *const c_char;

        // Convert C strings to Rust &str slices (handle null pointers)
        let a_str = if !a_ptr.is_null() {
            CStr::from_ptr(a_ptr).to_str().unwrap_or("")
        } else {
            ""
        };
        let b_str = if !b_ptr.is_null() {
            CStr::from_ptr(b_ptr).to_str().unwrap_or("")
        } else {
            ""
        };

        // Concatenate the two strings
        let result = format!("{}{}", a_str, b_str);

        // Allocate a new CString to hold the result
        let result_cstring = CString::new(result).unwrap();
        let result_ptr = result_cstring.into_raw();

        // Set the return object to the new string pointer
        asIScriptGeneric_SetReturnObject(g.as_ptr(), result_ptr as *mut std::ffi::c_void);
    }
}

fn string_length(g: &ScriptGeneric) {
    use std::ffi::CStr;
    use std::os::raw::{c_char, c_uint};

    unsafe {
        // Get the pointer to the string
        let self_ptr = asIScriptGeneric_GetObject(g.as_ptr()) as *const c_char;

        // Get the string length (handle null pointer gracefully)
        let len = if !self_ptr.is_null() {
            CStr::from_ptr(self_ptr).to_string_lossy().chars().count() as c_uint
        } else {
            0
        };

        // Set the return value
        let ret_ptr = asIScriptGeneric_GetAddressOfReturnLocation(g.as_ptr()) as *mut c_uint;
        if !ret_ptr.is_null() {
            *ret_ptr = len;
        }
    }
}

fn string_resize(g: &ScriptGeneric) {
    unsafe {
        // Get the pointer to the string (assume mutable, represented as CString or Vec<u8>)
        let self_ptr = asIScriptGeneric_GetObject(g.as_ptr()) as *mut String;
        // Get the pointer to the desired new length (asUINT)
        let len_ptr = asIScriptGeneric_GetAddressOfArg(g.as_ptr(), 0) as *const u32;

        if !self_ptr.is_null() && !len_ptr.is_null() {
            let self_str = &mut *self_ptr;
            let new_len = *len_ptr as usize;

            let curr_len = self_str.len();
            if new_len > curr_len {
                // Pad with '\0' or spaces to match C++'s std::string behavior
                self_str.push_str(&"\0".repeat(new_len - curr_len));
            } else {
                self_str.truncate(new_len);
            }
        }
    }
}

fn string_is_empty(g: &ScriptGeneric) {
    use std::ffi::CStr;
    use std::os::raw::c_char;

    unsafe {
        // Get the string pointer
        let self_ptr = asIScriptGeneric_GetObject(g.as_ptr()) as *const c_char;

        // Check if it's empty (handle null pointer)
        let is_empty = if !self_ptr.is_null() {
            let s = CStr::from_ptr(self_ptr).to_bytes();
            s.is_empty()
        } else {
            true
        };

        // Write to the return location
        let ret_ptr = asIScriptGeneric_GetAddressOfReturnLocation(g.as_ptr()) as *mut bool;
        if !ret_ptr.is_null() {
            *ret_ptr = is_empty;
        }
    }
}

fn string_char_at(g: &ScriptGeneric) {
    use std::os::raw::c_void;

    unsafe {
        // Get the requested index
        let index = asIScriptGeneric_GetArgDWord(g.as_ptr(), 0);

        // Get the string object; assumed to be a mutable pointer to String
        let self_ptr = asIScriptGeneric_GetObject(g.as_ptr()) as *mut String;
        if self_ptr.is_null() {
            asIScriptGeneric_SetReturnAddress(g.as_ptr(), std::ptr::null_mut());
            return;
        }

        let self_str = &mut *self_ptr;
        if (index as usize) >= self_str.len() {
            // Set a script exception
            let ctx = asGetActiveContext();
            if !ctx.is_null() {
                asContext_SetException(ctx, b"Out of range\0".as_ptr() as *const i8);
            }
            asIScriptGeneric_SetReturnAddress(g.as_ptr(), std::ptr::null_mut());
        } else {
            // Safe mutable access to the character at index as u8
            let char_ptr = self_str.as_mut_ptr().add(index as usize) as *mut c_void;
            asIScriptGeneric_SetReturnAddress(g.as_ptr(), char_ptr);
        }
    }
}

pub unsafe fn register_cstring(engine: &Engine) -> crate::error::Result<()> {
    engine.register_object_type(
        "string",
        std::mem::size_of::<*const c_char>(),
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
        CallConvTypes::asCALL_GENERIC,
    )?;

    engine.register_object_behaviour(
        "string",
        Behaviours::asBEHAVE_CONSTRUCT,
        "void f(const string &in)",
        copy_construct_string,
        CallConvTypes::asCALL_GENERIC,
    )?;

    engine.register_object_behaviour(
        "string",
        Behaviours::asBEHAVE_DESTRUCT,
        "void f()",
        destruct_string,
        CallConvTypes::asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "string &opAssign(const string &in)",
        assign_string,
        CallConvTypes::asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "string &opAddAssign(const string &in)",
        add_assign_string,
        CallConvTypes::asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "bool opEquals(const string &in) const",
        string_equals,
        CallConvTypes::asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "int opCmp(const string &in) const",
        string_cmp,
        CallConvTypes::asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "string opAdd(const string &in) const",
        string_add,
        CallConvTypes::asCALL_GENERIC,
    )?;

    // String methods. Add conditional methods as needed; here, the default.
    engine.register_object_method(
        "string",
        "uint length() const",
        string_length,
        CallConvTypes::asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "void resize(uint)",
        string_resize,
        CallConvTypes::asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "bool isEmpty() const",
        string_is_empty,
        CallConvTypes::asCALL_GENERIC,
    )?;

    // Indexing (mutator & inspector)
    engine.register_object_method(
        "string",
        "uint8 &opIndex(uint)",
        string_char_at,
        CallConvTypes::asCALL_GENERIC,
    )?;

    engine.register_object_method(
        "string",
        "uint8 &opIndex(uint) const",
        string_char_at,
        CallConvTypes::asCALL_GENERIC,
    )?;

    // Optionally implement the rest of the overloads as required

    // PRIMITIVE & PROPERTY OPs OMITTED (add if needed)

    // Register additional string methods (example)
    // chk!(asEngine_RegisterObjectMethod(
    //     engine,
    //     cstr("string"),
    //     cstr("string substr(uint start = 0, int count = -1) const"),
    //     Some(StringSubString_Generic),
    //     CallConvTypes::asCALL_GENERIC,
    // ));
    // // ...add additional methods in the same style...
    //
    // // Register global functions
    // chk!(asEngine_RegisterGlobalFunction(
    //     engine,
    //     cstr("string formatInt(int64 val, const string &in options = \"\", uint width = 0)"),
    //     Some(formatInt_Generic),
    //     CallConvTypes::asCALL_GENERIC,
    // ));
    // chk!(asEngine_RegisterGlobalFunction(
    //     engine,
    //     cstr("string formatUInt(uint64 val, const string &in options = \"\", uint width = 0)"),
    //     Some(formatUInt_Generic),
    //     CallConvTypes::asCALL_GENERIC,
    // ));
    // chk!(asEngine_RegisterGlobalFunction(
    //     engine,
    //     cstr(
    //         "string formatFloat(double val, const string &in options = \"\", uint width = 0, uint precision = 0)"
    //     ),
    //     Some(formatFloat_Generic),
    //     CallConvTypes::asCALL_GENERIC,
    // ));
    // chk!(asEngine_RegisterGlobalFunction(
    //     engine,
    //     cstr("int64 parseInt(const string &in, uint base = 10, uint &out byteCount = 0)"),
    //     Some(parseInt_Generic),
    //     CallConvTypes::asCALL_GENERIC,
    // ));
    // chk!(asEngine_RegisterGlobalFunction(
    //     engine,
    //     cstr("uint64 parseUInt(const string &in, uint base = 10, uint &out byteCount = 0)"),
    //     Some(parseUInt_Generic),
    //     CallConvTypes::asCALL_GENERIC,
    // ));
    // chk!(asEngine_RegisterGlobalFunction(
    //     engine,
    //     cstr("double parseFloat(const string &in, uint &out byteCount = 0)"),
    //     Some(parseFloat_Generic),
    //     CallConvTypes::asCALL_GENERIC,
    // ));

    Ok(())
}

unsafe impl Send for StringFactory {}
unsafe impl Sync for StringFactory {}
