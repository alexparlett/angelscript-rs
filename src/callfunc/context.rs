//! Function Call Context
//!
//! This module provides the context passed to native function calls.
//! It gives access to arguments, 'this' pointer, and return value.

use crate::core::types::ScriptValue;
use std::any::Any;

/// Context passed to native function calls
/// 
/// This provides access to:
/// - The 'this' object for methods (None for global functions)
/// - Arguments from the VM
/// - A way to set the return value
/// - Access to the heap for object operations
/// 
/// This is modeled after how AngelScript's CallSystemFunction prepares
/// and executes native calls.
pub struct FunctionCallContext<'a> {
    /// The 'this' object for methods (None for global functions)
    this_obj: Option<&'a mut dyn Any>,

    /// Arguments from the VM
    args: &'a [ScriptValue],

    /// Where to store the return value
    return_value: ScriptValue,
}

impl<'a> FunctionCallContext<'a> {
    /// Create a new function call context
    pub fn new(
        this_obj: Option<&'a mut dyn Any>,
        args: &'a [ScriptValue],
    ) -> Self {
        Self {
            this_obj,
            args,
            return_value: ScriptValue::Void,
        }
    }

    /// Get 'this' as a concrete type (mutable reference)
    /// 
    /// # Example
    /// ```ignore
    /// fn my_method(ctx: &mut FunctionCallContext) -> Result<(), String> {
    ///     let this = ctx.this_mut::<MyType>()?;
    ///     this.do_something();
    ///     Ok(())
    /// }
    /// ```
    pub fn this_mut<T: Any>(&mut self) -> Result<&mut T, String> {
        self.this_obj
            .as_mut()
            .ok_or_else(|| "Method requires 'this' but none provided".to_string())?
            .downcast_mut::<T>()
            .ok_or_else(|| format!(
                "'this' type mismatch: expected {}, got something else",
                std::any::type_name::<T>()
            ))
    }

    /// Get 'this' as a concrete type (immutable reference)
    /// 
    /// # Example
    /// ```ignore
    /// fn my_method(ctx: &mut FunctionCallContext) -> Result<(), String> {
    ///     let this = ctx.this_ref::<MyType>()?;
    ///     let value = this.get_value();
    ///     ctx.set_return(ScriptValue::Int32(value));
    ///     Ok(())
    /// }
    /// ```
    pub fn this_ref<T: Any>(&self) -> Result<&T, String> {
        self.this_obj
            .as_ref()
            .ok_or_else(|| "Method requires 'this' but none provided".to_string())?
            .downcast_ref::<T>()
            .ok_or_else(|| format!(
                "'this' type mismatch: expected {}, got something else",
                std::any::type_name::<T>()
            ))
    }

    /// Check if a 'this' object is provided
    pub fn has_this(&self) -> bool {
        self.this_obj.is_some()
    }

    /// Get argument by index
    pub fn arg(&self, index: usize) -> Option<&ScriptValue> {
        self.args.get(index)
    }

    /// Get argument count
    pub fn arg_count(&self) -> usize {
        self.args.len()
    }

    /// Get all arguments
    pub fn args(&self) -> &[ScriptValue] {
        self.args
    }

    /// Set the return value
    pub fn set_return(&mut self, value: ScriptValue) {
        self.return_value = value;
    }

    /// Set return value from a typed value
    pub fn return_value<T: Into<ScriptValue>>(&mut self, value: T) {
        self.return_value = value.into();
    }

    /// Consume the context and return the return value
    pub fn take_return_value(self) -> ScriptValue {
        self.return_value
    }

    /// Get the current return value (without consuming)
    pub fn get_return_value(&self) -> &ScriptValue {
        &self.return_value
    }

    // ========== Argument extraction helpers ==========

    /// Get argument as i32
    pub fn arg_i32(&self, index: usize) -> Result<i32, String> {
        self.arg(index)
            .ok_or_else(|| format!("Missing argument at index {}", index))?
            .as_i32()
            .ok_or_else(|| format!("Argument {} is not i32", index))
    }

    /// Get argument as bool
    pub fn arg_bool(&self, index: usize) -> Result<bool, String> {
        match self.arg(index) {
            Some(ScriptValue::Bool(b)) => Ok(*b),
            Some(_) => Err(format!("Argument {} is not bool", index)),
            None => Err(format!("Missing argument at index {}", index)),
        }
    }

    /// Get argument as f32
    pub fn arg_f32(&self, index: usize) -> Result<f32, String> {
        self.arg(index)
            .ok_or_else(|| format!("Missing argument at index {}", index))?
            .as_f32()
            .ok_or_else(|| format!("Argument {} is not f32", index))
    }

    /// Get argument as object handle
    pub fn arg_object_handle(&self, index: usize) -> Result<u64, String> {
        self.arg(index)
            .ok_or_else(|| format!("Missing argument at index {}", index))?
            .as_object_handle()
            .ok_or_else(|| format!("Argument {} is not object handle", index))
    }

    /// Get argument as string
    pub fn arg_string(&self, index: usize) -> Result<&str, String> {
        match self.arg(index) {
            Some(ScriptValue::String(s)) => Ok(s.as_str()),
            Some(_) => Err(format!("Argument {} is not string", index)),
            None => Err(format!("Missing argument at index {}", index)),
        }
    }
}

// ========== Conversions for common types to ScriptValue ==========

impl From<()> for ScriptValue {
    fn from(_: ()) -> Self {
        ScriptValue::Void
    }
}

impl From<bool> for ScriptValue {
    fn from(value: bool) -> Self {
        ScriptValue::Bool(value)
    }
}

impl From<i32> for ScriptValue {
    fn from(value: i32) -> Self {
        ScriptValue::Int32(value)
    }
}

impl From<i64> for ScriptValue {
    fn from(value: i64) -> Self {
        ScriptValue::Int64(value)
    }
}

impl From<u32> for ScriptValue {
    fn from(value: u32) -> Self {
        ScriptValue::UInt32(value)
    }
}

impl From<u64> for ScriptValue {
    fn from(value: u64) -> Self {
        ScriptValue::UInt64(value)
    }
}

impl From<f32> for ScriptValue {
    fn from(value: f32) -> Self {
        ScriptValue::Float(value)
    }
}

impl From<f64> for ScriptValue {
    fn from(value: f64) -> Self {
        ScriptValue::Double(value)
    }
}

impl From<String> for ScriptValue {
    fn from(value: String) -> Self {
        ScriptValue::String(value)
    }
}

impl From<&str> for ScriptValue {
    fn from(value: &str) -> Self {
        ScriptValue::String(value.to_string())
    }
}

impl From<usize> for ScriptValue {
    fn from(value: usize) -> Self {
        ScriptValue::UInt64(value as u64)
    }
}

impl From<Vec<u64>> for ScriptValue {
    fn from(value: Vec<u64>) -> Self {
        ScriptValue::InitList(value.into_iter().map(ScriptValue::ObjectHandle).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestType {
        value: i32,
    }

    #[test]
    fn test_context_no_this() {
        let args = vec![ScriptValue::Int32(42)];
        let ctx = FunctionCallContext::new(None, &args);

        assert!(!ctx.has_this());
        assert_eq!(ctx.arg_count(), 1);
    }

    #[test]
    fn test_context_with_this() {
        let mut obj = TestType { value: 10 };
        let args = vec![];
        let mut ctx = FunctionCallContext::new(Some(&mut obj), &args);

        assert!(ctx.has_this());
        
        let this = ctx.this_mut::<TestType>().unwrap();
        assert_eq!(this.value, 10);
        this.value = 20;
    }

    #[test]
    fn test_arg_extraction() {
        let args = vec![
            ScriptValue::Int32(42),
            ScriptValue::Bool(true),
            ScriptValue::Float(3.14),
        ];
        let ctx = FunctionCallContext::new(None, &args);

        assert_eq!(ctx.arg_i32(0).unwrap(), 42);
        assert_eq!(ctx.arg_bool(1).unwrap(), true);
        assert!((ctx.arg_f32(2).unwrap() - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_return_value() {
        let args = vec![];
        let mut ctx = FunctionCallContext::new(None, &args);

        ctx.set_return(ScriptValue::Int32(100));
        
        let result = ctx.take_return_value();
        assert!(matches!(result, ScriptValue::Int32(100)));
    }

    #[test]
    fn test_conversions() {
        let _: ScriptValue = 42i32.into();
        let _: ScriptValue = true.into();
        let _: ScriptValue = 3.14f32.into();
        let _: ScriptValue = "hello".into();
    }
}