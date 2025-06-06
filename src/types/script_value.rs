use crate::types::script_memory::ScriptMemoryLocation;
use crate::prelude::{ScriptGeneric, TypeIdFlags};
use angelscript_sys::{asBYTE, asDWORD, asQWORD, asUINT, asWORD};

// Expand the GenericValueData enum to handle more cases
#[derive(Debug, Clone)]
pub enum ScriptValue {
    // Primitive types
    Byte(asBYTE),
    Word(asWORD),
    DWord(asDWORD),
    QWord(asQWORD),
    Float(f32),
    Double(f64),

    // Pointer types
    Address(ScriptMemoryLocation), // Reference to primitive or object
    Object(ScriptMemoryLocation),  // Object by value
    ObjectHandle(ScriptMemoryLocation), // Object handle (pointer to object)
    AppObject(ScriptMemoryLocation), // Application registered object
    ScriptObject(ScriptMemoryLocation), // Script object

    // Special cases
    Null,         // Null pointer/handle
    Unknown(i32), // Unknown type with type_id
}

impl ScriptValue {
    /// Creates a GenericValueData from a ScriptGeneric argument using proper AngelScript type IDs
    pub fn from_generic(generic: &ScriptGeneric, arg: asUINT, flags: TypeIdFlags) -> Self {
        // Handle primitive types first
        match flags {
            TypeIdFlags::VOID => ScriptValue::DWord(0), // void shouldn't happen for args
            TypeIdFlags::BOOL => ScriptValue::Byte(generic.get_arg_byte(arg)),
            TypeIdFlags::INT8 => ScriptValue::Byte(generic.get_arg_byte(arg)),
            TypeIdFlags::INT16 => ScriptValue::Word(generic.get_arg_word(arg)),
            TypeIdFlags::INT32 => ScriptValue::DWord(generic.get_arg_dword(arg)),
            TypeIdFlags::INT64 => ScriptValue::QWord(generic.get_arg_qword(arg)),
            TypeIdFlags::UINT8 => ScriptValue::Byte(generic.get_arg_byte(arg)),
            TypeIdFlags::UINT16 => ScriptValue::Word(generic.get_arg_word(arg)),
            TypeIdFlags::UINT32 => ScriptValue::DWord(generic.get_arg_dword(arg)),
            TypeIdFlags::UINT64 => ScriptValue::QWord(generic.get_arg_qword(arg)),
            TypeIdFlags::FLOAT => ScriptValue::Float(generic.get_arg_float(arg)),
            TypeIdFlags::DOUBLE => ScriptValue::Double(generic.get_arg_double(arg)),
            _ => {
                // Handle complex types using type flags
                Self::from_complex_type(generic, arg, flags)
            }
        }
    }

    /// Handles complex types (objects, handles, references, etc.)
    fn from_complex_type(generic: &ScriptGeneric, arg: asUINT, flags: TypeIdFlags) -> Self {
        // Check if it's a handle (pointer to object)
        if (flags & TypeIdFlags::OBJHANDLE) != TypeIdFlags::VOID {
            // It's an object handle
            if let Some(ptr) = generic.get_arg_object(arg) {
                ScriptValue::ObjectHandle(ptr)
            } else {
                ScriptValue::ObjectHandle(ScriptMemoryLocation::null())
            }
        }
        // Check if it's an object (by value or reference)
        else if (flags & TypeIdFlags::MASK_OBJECT) != TypeIdFlags::VOID {
            // Determine if it's an application object or script object
            if (flags & TypeIdFlags::APPOBJECT) != TypeIdFlags::VOID {
                // Application registered object
                if let Some(ptr) = generic.get_arg_address(arg) {
                    ScriptValue::AppObject(ptr)
                } else if let Some(ptr) = generic.get_arg_object(arg) {
                    ScriptValue::AppObject(ptr)
                } else {
                    ScriptValue::AppObject(ScriptMemoryLocation::null())
                }
            } else if (flags & TypeIdFlags::SCRIPTOBJECT) != TypeIdFlags::VOID {
                // Script object
                if let Some(ptr) = generic.get_arg_object(arg) {
                    ScriptValue::ScriptObject(ptr)
                } else {
                    ScriptValue::ScriptObject(ScriptMemoryLocation::null())
                }
            } else {
                // Generic object
                if let Some(ptr) = generic.get_arg_address(arg) {
                    ScriptValue::Address(ptr)
                } else if let Some(ptr) = generic.get_arg_object(arg) {
                    ScriptValue::Object(ptr)
                } else {
                    ScriptValue::Object(ScriptMemoryLocation::null())
                }
            }
        }
        // Handle references and other special cases
        else {
            // Try to get as address first (for references)
            if let Some(ptr) = generic.get_arg_address(arg) {
                ScriptValue::Address(ptr)
            } else {
                // Fallback to treating as a primitive value
                ScriptValue::DWord(generic.get_arg_dword(arg))
            }
        }
    }

    /// Gets the actual value based on the known Rust type
    pub fn get_as<T>(&self) -> Option<T>
    where
        T: FromScriptValue,
    {
        T::from_script_value(self)
    }
}

/// Trait for converting from GenericValueData to specific Rust types
pub trait FromScriptValue: Sized {
    fn from_script_value(value: &ScriptValue) -> Option<Self>;
}

// Implementations for basic types
impl FromScriptValue for bool {
    fn from_script_value(value: &ScriptValue) -> Option<Self> {
        match value {
            ScriptValue::Byte(b) => Some(*b != 0),
            ScriptValue::DWord(d) => Some(*d != 0),
            _ => None,
        }
    }
}

impl FromScriptValue for i8 {
    fn from_script_value(value: &ScriptValue) -> Option<Self> {
        match value {
            ScriptValue::Byte(b) => Some(*b as i8),
            _ => None,
        }
    }
}

impl FromScriptValue for i32 {
    fn from_script_value(value: &ScriptValue) -> Option<Self> {
        match value {
            ScriptValue::DWord(d) => Some(*d as i32),
            ScriptValue::Word(w) => Some(*w as i32),
            ScriptValue::Byte(b) => Some(*b as i32),
            _ => None,
        }
    }
}

impl FromScriptValue for f32 {
    fn from_script_value(value: &ScriptValue) -> Option<Self> {
        match value {
            ScriptValue::Float(f) => Some(*f),
            _ => None,
        }
    }
}

impl FromScriptValue for f64 {
    fn from_script_value(value: &ScriptValue) -> Option<Self> {
        match value {
            ScriptValue::Double(d) => Some(*d),
            ScriptValue::Float(f) => Some(*f as f64),
            _ => None,
        }
    }
}

// ========== GENERIC VALUE REPRESENTATION ==========

/// Represents a generic value with type information
#[derive(Debug, Clone)]
pub struct ScriptArg {
    pub type_id: i32,
    pub flags: TypeIdFlags,
    pub value: ScriptValue,
}
