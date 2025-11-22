//! Function Wrappers
//!
//! This module provides helper functions to wrap typed Rust closures
//! into SystemFunctionInterface instances that can be registered with the engine.
//!
//! These helpers handle the boilerplate of downcasting 'this' pointers and
//! converting between Rust types and ScriptValues.

use super::context::FunctionCallContext;
use super::interface::{CallConv, NativeCallable, ParamType, ReturnType, SystemFunctionInterface};
use crate::core::types::ScriptValue;
use std::any::Any;
use std::sync::Arc;

// ============================================================================
// Method wrappers (for types that implement methods)
// ============================================================================

/// Wrap a method with no arguments
/// 
/// # Example
/// ```ignore
/// struct Player { health: i32 }
/// 
/// let interface = wrap_method::<Player, _, _>(
///     |this| this.health,
///     "get_health"
/// );
/// ```
pub fn wrap_method<T, F, R>(func: F, name: impl Into<String>) -> SystemFunctionInterface
where
    T: Any + Send + Sync,
    F: Fn(&mut T) -> R + Send + Sync + 'static,
    R: Into<ScriptValue>,
{
    SystemFunctionInterface::new(
        NativeCallable::Generic(Arc::new(move |ctx: &mut FunctionCallContext| {
            let this = ctx.this_mut::<T>()?;
            let result = func(this);
            ctx.set_return(result.into());
            Ok(())
        })),
        CallConv::ThisCall,
        Some(std::any::TypeId::of::<T>()),
        vec![],
        ReturnType::Dynamic,
        name,
    )
}

/// Wrap a method that returns Result
/// 
/// # Example
/// ```ignore
/// let interface = wrap_method_result::<Player, _, _>(
///     |this| {
///         if this.health > 0 { Ok(this.health) }
///         else { Err("Player is dead".to_string()) }
///     },
///     "get_health_checked"
/// );
/// ```
pub fn wrap_method_result<T, F, R>(func: F, name: impl Into<String>) -> SystemFunctionInterface
where
    T: Any + Send + Sync,
    F: Fn(&mut T) -> Result<R, String> + Send + Sync + 'static,
    R: Into<ScriptValue>,
{
    SystemFunctionInterface::new(
        NativeCallable::Generic(Arc::new(move |ctx: &mut FunctionCallContext| {
            let this = ctx.this_mut::<T>()?;
            let result = func(this)?;
            ctx.set_return(result.into());
            Ok(())
        })),
        CallConv::ThisCall,
        Some(std::any::TypeId::of::<T>()),
        vec![],
        ReturnType::Dynamic,
        name,
    )
}

/// Wrap a method that takes a slice of arguments
/// 
/// # Example
/// ```ignore
/// let interface = wrap_method_with_args::<Player, _, _>(
///     |this, args| {
///         let damage = args.get(0)?.as_i32()?;
///         this.health -= damage;
///         this.health
///     },
///     1,
///     "take_damage"
/// );
/// ```
pub fn wrap_method_with_args<T, F, R>(
    func: F,
    param_count: usize,
    name: impl Into<String>,
) -> SystemFunctionInterface
where
    T: Any + Send + Sync,
    F: Fn(&mut T, &[ScriptValue]) -> R + Send + Sync + 'static,
    R: Into<ScriptValue>,
{
    SystemFunctionInterface::new(
        NativeCallable::Generic(Arc::new(move |ctx: &mut FunctionCallContext| {
            let args = ctx.args();
            let this = ctx.this_mut::<T>()?;
            let result = func(this, args);
            ctx.set_return(result.into());
            Ok(())
        })),
        CallConv::ThisCall,
        Some(std::any::TypeId::of::<T>()),
        vec![ParamType::Any; param_count],
        ReturnType::Dynamic,
        name,
    )
}

/// Wrap a void method (no return value)
pub fn wrap_method_void<T, F>(func: F, name: impl Into<String>) -> SystemFunctionInterface
where
    T: Any + Send + Sync,
    F: Fn(&mut T) + Send + Sync + 'static,
{
    SystemFunctionInterface::new(
        NativeCallable::Generic(Arc::new(move |ctx: &mut FunctionCallContext| {
            let this = ctx.this_mut::<T>()?;
            func(this);
            ctx.set_return(ScriptValue::Void);
            Ok(())
        })),
        CallConv::ThisCall,
        Some(std::any::TypeId::of::<T>()),
        vec![],
        ReturnType::Void,
        name,
    )
}

/// Wrap a const method (immutable reference to self)
pub fn wrap_method_const<T, F, R>(func: F, name: impl Into<String>) -> SystemFunctionInterface
where
    T: Any + Send + Sync,
    F: Fn(&T) -> R + Send + Sync + 'static,
    R: Into<ScriptValue>,
{
    SystemFunctionInterface::new(
        NativeCallable::Generic(Arc::new(move |ctx: &mut FunctionCallContext| {
            let this = ctx.this_ref::<T>()?;
            let result = func(this);
            ctx.set_return(result.into());
            Ok(())
        })),
        CallConv::ThisCall,
        Some(std::any::TypeId::of::<T>()),
        vec![],
        ReturnType::Dynamic,
        name,
    )
}

// ============================================================================
// Global function wrappers
// ============================================================================

/// Wrap a global function with no arguments
/// 
/// # Example
/// ```ignore
/// let interface = wrap_global(|| 42, "get_answer");
/// ```
pub fn wrap_global<F, R>(func: F, name: impl Into<String>) -> SystemFunctionInterface
where
    F: Fn() -> R + Send + Sync + 'static,
    R: Into<ScriptValue>,
{
    SystemFunctionInterface::new(
        NativeCallable::Generic(Arc::new(move |ctx: &mut FunctionCallContext| {
            let result = func();
            ctx.set_return(result.into());
            Ok(())
        })),
        CallConv::CDecl,
        None,
        vec![],
        ReturnType::Dynamic,
        name,
    )
}

/// Wrap a global function that takes arguments
/// 
/// # Example
/// ```ignore
/// let interface = wrap_global_with_args(
///     |args| {
///         let a = args[0].as_i32().unwrap_or(0);
///         let b = args[1].as_i32().unwrap_or(0);
///         a + b
///     },
///     2,
///     "add"
/// );
/// ```
pub fn wrap_global_with_args<F, R>(
    func: F,
    param_count: usize,
    name: impl Into<String>,
) -> SystemFunctionInterface
where
    F: Fn(&[ScriptValue]) -> R + Send + Sync + 'static,
    R: Into<ScriptValue>,
{
    SystemFunctionInterface::new(
        NativeCallable::Generic(Arc::new(move |ctx: &mut FunctionCallContext| {
            let result = func(ctx.args());
            ctx.set_return(result.into());
            Ok(())
        })),
        CallConv::CDecl,
        None,
        vec![ParamType::Any; param_count],
        ReturnType::Dynamic,
        name,
    )
}

/// Wrap a global function that returns Result
pub fn wrap_global_result<F, R>(func: F, name: impl Into<String>) -> SystemFunctionInterface
where
    F: Fn() -> Result<R, String> + Send + Sync + 'static,
    R: Into<ScriptValue>,
{
    SystemFunctionInterface::new(
        NativeCallable::Generic(Arc::new(move |ctx: &mut FunctionCallContext| {
            let result = func()?;
            ctx.set_return(result.into());
            Ok(())
        })),
        CallConv::CDecl,
        None,
        vec![],
        ReturnType::Dynamic,
        name,
    )
}

/// Wrap a global function that takes arguments and returns Result
pub fn wrap_global_with_args_result<F, R>(
    func: F,
    param_count: usize,
    name: impl Into<String>,
) -> SystemFunctionInterface
where
    F: Fn(&[ScriptValue]) -> Result<R, String> + Send + Sync + 'static,
    R: Into<ScriptValue>,
{
    SystemFunctionInterface::new(
        NativeCallable::Generic(Arc::new(move |ctx: &mut FunctionCallContext| {
            let result = func(ctx.args())?;
            ctx.set_return(result.into());
            Ok(())
        })),
        CallConv::CDecl,
        None,
        vec![ParamType::Any; param_count],
        ReturnType::Dynamic,
        name,
    )
}

// ============================================================================
// Low-level wrapper for raw context functions
// ============================================================================

/// Wrap a raw context function
/// 
/// This is the lowest level wrapper - you get full control over the context.
/// 
/// # Example
/// ```ignore
/// let interface = wrap_raw(
///     |ctx| {
///         let this = ctx.this_mut::<MyType>()?;
///         let arg = ctx.arg_i32(0)?;
///         this.do_something(arg);
///         ctx.set_return(ScriptValue::Bool(true));
///         Ok(())
///     },
///     Some(TypeId::of::<MyType>()),
///     "do_something"
/// );
/// ```
pub fn wrap_raw<F>(
    func: F,
    this_type: Option<std::any::TypeId>,
    name: impl Into<String>,
) -> SystemFunctionInterface
where
    F: Fn(&mut FunctionCallContext) -> Result<(), String> + Send + Sync + 'static,
{
    let call_conv = if this_type.is_some() {
        CallConv::ThisCall
    } else {
        CallConv::CDecl
    };

    SystemFunctionInterface::new(
        NativeCallable::Generic(Arc::new(func)),
        call_conv,
        this_type,
        vec![],
        ReturnType::Dynamic,
        name,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestObject {
        value: i32,
    }

    #[test]
    fn test_wrap_method() {
        let interface = wrap_method::<TestObject, _, _>(
            |this| this.value,
            "get_value",
        );

        assert!(interface.is_method());
        assert_eq!(interface.name, "get_value");
        assert_eq!(interface.call_conv, CallConv::ThisCall);
    }

    #[test]
    fn test_wrap_method_void() {
        let interface = wrap_method_void::<TestObject, _>(
            |this| { this.value = 0; },
            "reset",
        );

        assert!(interface.is_method());
        assert_eq!(interface.return_type, ReturnType::Void);
    }

    #[test]
    fn test_wrap_global() {
        let interface = wrap_global(|| 42i32, "get_answer");

        assert!(interface.is_global());
        assert_eq!(interface.name, "get_answer");
        assert_eq!(interface.call_conv, CallConv::CDecl);
    }

    #[test]
    fn test_wrap_global_with_args() {
        let interface = wrap_global_with_args(
            |args| {
                let a = args.get(0).and_then(|v| v.as_i32()).unwrap_or(0);
                let b = args.get(1).and_then(|v| v.as_i32()).unwrap_or(0);
                a + b
            },
            2,
            "add",
        );

        assert!(interface.is_global());
        assert_eq!(interface.param_types.len(), 2);
    }

    #[test]
    fn test_call_wrapped_method() {
        let interface = wrap_method::<TestObject, _, _>(
            |this| {
                this.value += 10;
                this.value
            },
            "increment",
        );

        let mut obj = TestObject { value: 5 };
        let mut ctx = FunctionCallContext::new(Some(&mut obj), &[]);

        match &interface.func {
            NativeCallable::Generic(f) => {
                f(&mut ctx).unwrap();
            }
        }

        let result = ctx.take_return_value();
        assert!(matches!(result, ScriptValue::Int32(15)));
    }

    #[test]
    fn test_call_wrapped_global() {
        let interface = wrap_global_with_args(
            |args| {
                let a = args.get(0).and_then(|v| v.as_i32()).unwrap_or(0);
                let b = args.get(1).and_then(|v| v.as_i32()).unwrap_or(0);
                a * b
            },
            2,
            "multiply",
        );

        let args = vec![ScriptValue::Int32(7), ScriptValue::Int32(6)];
        let mut ctx = FunctionCallContext::new(None, &args);

        match &interface.func {
            NativeCallable::Generic(f) => {
                f(&mut ctx).unwrap();
            }
        }

        let result = ctx.take_return_value();
        assert!(matches!(result, ScriptValue::Int32(42)));
    }
}