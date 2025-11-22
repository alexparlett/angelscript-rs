//! System Function Interface
//!
//! This module defines the core types for describing how to call native functions.
//! Equivalent to `asSSystemFunctionInterface` in C++ AngelScript.

use std::sync::Arc;

/// Calling convention for native functions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallConv {
    /// Global function: fn(args...) -> R
    CDecl,
    /// Method with this as receiver: fn(&mut self, args...) -> R
    ThisCall,
    /// Method with this last: fn(args..., this) -> R
    CDeclObjLast,
    /// Method with this first (cdecl style): fn(this, args...) -> R
    CDeclObjFirst,
    /// Generic calling convention (our default for Rust closures)
    Generic,
}

impl Default for CallConv {
    fn default() -> Self {
        CallConv::Generic
    }
}

/// Parameter type information for marshalling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamType {
    /// Any type (runtime checked)
    Any,
    /// Void (for no parameters)
    Void,
    /// Boolean
    Bool,
    /// 8-bit signed integer
    Int8,
    /// 16-bit signed integer
    Int16,
    /// 32-bit signed integer
    Int32,
    /// 64-bit signed integer
    Int64,
    /// 8-bit unsigned integer
    UInt8,
    /// 16-bit unsigned integer
    UInt16,
    /// 32-bit unsigned integer
    UInt32,
    /// 64-bit unsigned integer
    UInt64,
    /// 32-bit float
    Float,
    /// 64-bit double
    Double,
    /// String
    String,
    /// Object handle (reference to heap object)
    ObjectHandle,
}

impl Default for ParamType {
    fn default() -> Self {
        ParamType::Any
    }
}

/// Return type information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnType {
    /// Void (no return)
    Void,
    /// Boolean
    Bool,
    /// 32-bit signed integer
    Int32,
    /// 64-bit signed integer
    Int64,
    /// 32-bit unsigned integer
    UInt32,
    /// 64-bit unsigned integer
    UInt64,
    /// 32-bit float
    Float,
    /// 64-bit double
    Double,
    /// String
    String,
    /// Object handle
    ObjectHandle,
    /// Dynamic type (runtime determined)
    Dynamic,
}

impl Default for ReturnType {
    fn default() -> Self {
        ReturnType::Void
    }
}

impl ReturnType {
    /// Infer return type from a Rust type
    pub fn infer<T: 'static>() -> Self {
        use std::any::TypeId;

        let type_id = TypeId::of::<T>();

        if type_id == TypeId::of::<()>() {
            ReturnType::Void
        } else if type_id == TypeId::of::<bool>() {
            ReturnType::Bool
        } else if type_id == TypeId::of::<i32>() {
            ReturnType::Int32
        } else if type_id == TypeId::of::<i64>() {
            ReturnType::Int64
        } else if type_id == TypeId::of::<u32>() {
            ReturnType::UInt32
        } else if type_id == TypeId::of::<u64>() {
            ReturnType::UInt64
        } else if type_id == TypeId::of::<f32>() {
            ReturnType::Float
        } else if type_id == TypeId::of::<f64>() {
            ReturnType::Double
        } else if type_id == TypeId::of::<String>() {
            ReturnType::String
        } else {
            ReturnType::Dynamic
        }
    }
}

/// The signature type for native callable functions
/// 
/// This takes a FunctionCallContext and returns a Result indicating success or error.
pub type NativeCallFn = dyn Fn(&mut super::context::FunctionCallContext) -> Result<(), String> + Send + Sync;

/// Type-erased callable wrapper
/// 
/// This is what actually gets invoked when a system function is called.
/// It wraps the actual Rust closure/function in a type-erased form.
pub enum NativeCallable {
    /// A generic callable that takes a FunctionCallContext
    Generic(Arc<NativeCallFn>),
}

impl std::fmt::Debug for NativeCallable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NativeCallable::Generic(_) => write!(f, "NativeCallable::Generic(...)"),
        }
    }
}

impl Clone for NativeCallable {
    fn clone(&self) -> Self {
        match self {
            NativeCallable::Generic(f) => NativeCallable::Generic(Arc::clone(f)),
        }
    }
}

/// System function interface - describes how to call a native function
/// 
/// This is equivalent to `asSSystemFunctionInterface` in C++ AngelScript.
/// Each registered native function has one of these stored in the registry.
#[derive(Clone)]
pub struct SystemFunctionInterface {
    /// The actual callable
    pub func: NativeCallable,

    /// Calling convention
    pub call_conv: CallConv,

    /// The Rust TypeId of the 'this' type (for methods)
    /// None for global functions
    pub this_type: Option<std::any::TypeId>,

    /// Parameter type information for marshalling
    pub param_types: Vec<ParamType>,

    /// Return type information
    pub return_type: ReturnType,

    /// Human-readable name for debugging
    pub name: String,
}

impl std::fmt::Debug for SystemFunctionInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemFunctionInterface")
            .field("call_conv", &self.call_conv)
            .field("this_type", &self.this_type)
            .field("param_types", &self.param_types)
            .field("return_type", &self.return_type)
            .field("name", &self.name)
            .finish()
    }
}

impl SystemFunctionInterface {
    /// Create a new system function interface
    pub fn new(
        func: NativeCallable,
        call_conv: CallConv,
        this_type: Option<std::any::TypeId>,
        param_types: Vec<ParamType>,
        return_type: ReturnType,
        name: impl Into<String>,
    ) -> Self {
        Self {
            func,
            call_conv,
            this_type,
            param_types,
            return_type,
            name: name.into(),
        }
    }

    /// Check if this is a method (has a 'this' type)
    pub fn is_method(&self) -> bool {
        self.this_type.is_some()
    }

    /// Check if this is a global function (no 'this' type)
    pub fn is_global(&self) -> bool {
        self.this_type.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_return_type_infer() {
        assert_eq!(ReturnType::infer::<()>(), ReturnType::Void);
        assert_eq!(ReturnType::infer::<bool>(), ReturnType::Bool);
        assert_eq!(ReturnType::infer::<i32>(), ReturnType::Int32);
        assert_eq!(ReturnType::infer::<f32>(), ReturnType::Float);
        assert_eq!(ReturnType::infer::<String>(), ReturnType::String);
    }

    #[test]
    fn test_call_conv_default() {
        assert_eq!(CallConv::default(), CallConv::Generic);
    }
}