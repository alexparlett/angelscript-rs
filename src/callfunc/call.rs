//! System Function Call
//!
//! This module provides the unified entry point for calling native functions.
//! Equivalent to `CallSystemFunction()` in C++ AngelScript's as_callfunc.cpp.

use super::context::FunctionCallContext;
use super::interface::NativeCallable;
use super::registry::SystemFunctionRegistry;
use crate::core::types::{FunctionId, ScriptValue};
use std::any::Any;

/// Call a system function
/// 
/// This is THE unified entry point for calling any native function.
/// Both the VM (via CALLSYS instruction) and the GC (via behaviours) use this.
/// 
/// # Arguments
/// 
/// * `func_id` - The function ID to call
/// * `this_obj` - The 'this' object for methods, or None for global functions
/// * `args` - Arguments to pass to the function
/// * `registry` - The system function registry containing the function
/// 
/// # Returns
/// 
/// The return value from the function, or an error.
/// 
/// # Example
/// 
/// ```ignore
/// // VM calling a system function
/// let result = call_system_function(
///     func_id,
///     Some(&mut object),
///     &args,
///     &registry,
/// )?;
/// ```
pub fn call_system_function(
    func_id: FunctionId,
    this_obj: Option<&mut dyn Any>,
    args: &[ScriptValue],
    registry: &SystemFunctionRegistry,
) -> Result<ScriptValue, String> {
    // 1. Look up the function interface
    let sys_func = registry.get(func_id).ok_or_else(|| {
        format!("System function {} not found in registry", func_id)
    })?;

    // 2. Validate 'this' pointer is provided if required
    // Note: Actual type checking happens in the callable via downcast
    if sys_func.this_type.is_some() && this_obj.is_none() {
        return Err(format!(
            "Method '{}' requires 'this' but none provided",
            sys_func.name
        ));
    }

    // 3. Create call context
    let mut ctx = FunctionCallContext::new(this_obj, args);

    // 4. Call the function
    match &sys_func.func {
        NativeCallable::Generic(f) => {
            f(&mut ctx)?;
        }
    }

    // 5. Return the result
    Ok(ctx.take_return_value())
}

/// Call a system function with a raw callable (bypasses registry lookup)
/// 
/// This is useful for calling behaviours that you already have a reference to.
pub fn call_system_function_direct(
    callable: &NativeCallable,
    this_obj: Option<&mut dyn Any>,
    args: &[ScriptValue],
) -> Result<ScriptValue, String> {
    let mut ctx = FunctionCallContext::new(this_obj, args);

    match callable {
        NativeCallable::Generic(f) => {
            f(&mut ctx)?;
        }
    }

    Ok(ctx.take_return_value())
}

/// Helper struct for building and executing a system call
/// 
/// This provides a builder pattern for more complex call scenarios.
pub struct SystemCall<'a> {
    registry: &'a SystemFunctionRegistry,
    func_id: FunctionId,
    this_obj: Option<&'a mut dyn Any>,
    args: Vec<ScriptValue>,
}

impl<'a> SystemCall<'a> {
    /// Create a new system call builder
    pub fn new(registry: &'a SystemFunctionRegistry, func_id: FunctionId) -> Self {
        Self {
            registry,
            func_id,
            this_obj: None,
            args: Vec::new(),
        }
    }

    /// Set the 'this' object for method calls
    pub fn with_this(mut self, this: &'a mut dyn Any) -> Self {
        self.this_obj = Some(this);
        self
    }

    /// Add an argument
    pub fn arg(mut self, value: impl Into<ScriptValue>) -> Self {
        self.args.push(value.into());
        self
    }

    /// Add multiple arguments
    pub fn args(mut self, values: impl IntoIterator<Item = ScriptValue>) -> Self {
        self.args.extend(values);
        self
    }

    /// Execute the call
    pub fn call(self) -> Result<ScriptValue, String> {
        call_system_function(
            self.func_id,
            self.this_obj,
            &self.args,
            self.registry,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::interface::{CallConv, ReturnType, SystemFunctionInterface};
    use std::sync::Arc;

    struct TestObject {
        value: i32,
    }

    #[test]
    fn test_call_global_function() {
        let mut registry = SystemFunctionRegistry::new();
        
        // Register a simple global function
        let interface = SystemFunctionInterface::new(
            NativeCallable::Generic(Arc::new(|ctx| {
                let a = ctx.arg_i32(0)?;
                let b = ctx.arg_i32(1)?;
                ctx.set_return(ScriptValue::Int32(a + b));
                Ok(())
            })),
            CallConv::CDecl,
            None,
            vec![],
            ReturnType::Int32,
            "add",
        );
        registry.register(1000, interface);

        let args = vec![ScriptValue::Int32(10), ScriptValue::Int32(20)];
        let result = call_system_function(1000, None, &args, &registry).unwrap();

        assert!(matches!(result, ScriptValue::Int32(30)));
    }

    #[test]
    fn test_call_method() {
        let mut registry = SystemFunctionRegistry::new();
        
        // Register a method
        let interface = SystemFunctionInterface::new(
            NativeCallable::Generic(Arc::new(|ctx| {
                let this = ctx.this_ref::<TestObject>()?;
                ctx.set_return(ScriptValue::Int32(this.value));
                Ok(())
            })),
            CallConv::ThisCall,
            Some(std::any::TypeId::of::<TestObject>()),
            vec![],
            ReturnType::Int32,
            "get_value",
        );
        registry.register(1001, interface);

        let mut obj = TestObject { value: 42 };
        let result = call_system_function(
            1001,
            Some(&mut obj),
            &[],
            &registry,
        ).unwrap();

        assert!(matches!(result, ScriptValue::Int32(42)));
    }

    #[test]
    fn test_call_mutating_method() {
        let mut registry = SystemFunctionRegistry::new();
        
        // Register a mutating method
        let interface = SystemFunctionInterface::new(
            NativeCallable::Generic(Arc::new(|ctx| {
                let new_value = ctx.arg_i32(0)?;
                let this = ctx.this_mut::<TestObject>()?;
                this.value = new_value;
                Ok(())
            })),
            CallConv::ThisCall,
            Some(std::any::TypeId::of::<TestObject>()),
            vec![],
            ReturnType::Void,
            "set_value",
        );
        registry.register(1002, interface);

        let mut obj = TestObject { value: 0 };
        call_system_function(
            1002,
            Some(&mut obj),
            &[ScriptValue::Int32(100)],
            &registry,
        ).unwrap();

        assert_eq!(obj.value, 100);
    }

    #[test]
    fn test_missing_function() {
        let registry = SystemFunctionRegistry::new();
        
        let result = call_system_function(9999, None, &[], &registry);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_method_without_this() {
        let mut registry = SystemFunctionRegistry::new();
        
        // Register a method that expects 'this'
        let interface = SystemFunctionInterface::new(
            NativeCallable::Generic(Arc::new(|_ctx| Ok(()))),
            CallConv::ThisCall,
            Some(std::any::TypeId::of::<TestObject>()),
            vec![],
            ReturnType::Void,
            "method",
        );
        registry.register(1003, interface);

        // Call without providing 'this'
        let result = call_system_function(1003, None, &[], &registry);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires 'this'"));
    }

    #[test]
    fn test_system_call_builder() {
        let mut registry = SystemFunctionRegistry::new();
        
        let interface = SystemFunctionInterface::new(
            NativeCallable::Generic(Arc::new(|ctx| {
                let a = ctx.arg_i32(0)?;
                let b = ctx.arg_i32(1)?;
                ctx.set_return(ScriptValue::Int32(a * b));
                Ok(())
            })),
            CallConv::CDecl,
            None,
            vec![],
            ReturnType::Int32,
            "multiply",
        );
        registry.register(1004, interface);

        let result = SystemCall::new(&registry, 1004)
            .arg(5i32)
            .arg(6i32)
            .call()
            .unwrap();

        assert!(matches!(result, ScriptValue::Int32(30)));
    }
}