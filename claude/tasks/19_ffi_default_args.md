# Task 19: FFI Default Arguments

## Problem Summary

FFI functions can be registered with default argument values using `FfiExpr`, but these defaults are never compiled to bytecode. When a script calls an FFI function with fewer arguments than parameters, the compiler currently emits an error:

```
FFI default arguments not yet supported at compile time
```

## Current State

- `Param` in `angelscript-core` has `has_default: bool` but no actual default value storage
- `FfiExpr` exists in `angelscript-core` and can represent default values (Int, Float, String, etc.)
- Script functions store defaults as `Option<&'ast Expr<'ast>>` in `ScriptParam`
- The broken code is in `src/semantic/passes/function_processor/expr_checker.rs` lines 1833-1867

## Dependencies

This task depends on the new compiler passes being built (Tasks 8-15 of Task 26). Once the new `ExpressionChecker` exists in `crates/angelscript-compiler`, we can implement FFI default argument support cleanly.

## Solution

### 1. Add FFI Default Value Storage

Store FFI default values in `FfiRegistry`:

```rust
// In angelscript-ffi/src/registry/ffi_registry.rs
pub struct FfiRegistry {
    // ... existing fields ...

    /// FFI default argument values: (func_hash, param_idx) → FfiExpr
    ffi_defaults: FxHashMap<(TypeHash, usize), FfiExpr>,
}

impl FfiRegistry {
    /// Get the default value for an FFI function parameter.
    pub fn get_default_value(&self, func_hash: TypeHash, param_idx: usize) -> Option<&FfiExpr> {
        self.ffi_defaults.get(&(func_hash, param_idx))
    }
}
```

### 2. Update FfiRegistryBuilder

Add method to register default values:

```rust
impl FfiRegistryBuilder {
    /// Register an FFI function with default argument values.
    pub fn register_function_with_defaults(
        &mut self,
        func: FunctionDef,
        native_fn: Option<NativeFn>,
        defaults: Vec<(usize, FfiExpr)>,  // (param_idx, default_value)
    ) {
        let func_hash = func.func_hash;
        self.register_function(func, native_fn);
        for (idx, expr) in defaults {
            self.ffi_defaults.insert((func_hash, idx), expr);
        }
    }
}
```

### 3. Add DefaultValue Enum to CompilationContext

In the new compiler's `CompilationContext`:

```rust
/// Default value for a function parameter.
pub enum DefaultValue<'a, 'ast> {
    /// Script function default (AST expression)
    Script(&'ast Expr<'ast>),
    /// FFI function default (owned expression)
    Ffi(&'a FfiExpr),
}

impl CompilationContext {
    /// Get the default value for a function parameter.
    pub fn get_default_value(
        &self,
        func_hash: TypeHash,
        param_idx: usize,
    ) -> Option<DefaultValue<'_, '_>> {
        // Try FFI first
        if let Some(ffi_expr) = self.ffi.get_default_value(func_hash, param_idx) {
            return Some(DefaultValue::Ffi(ffi_expr));
        }
        // Try script
        if let Some(script_func) = self.script.get_function(func_hash) {
            if let Some(default) = script_func.params.get(param_idx)?.default {
                return Some(DefaultValue::Script(default));
            }
        }
        None
    }
}
```

### 4. Implement FfiExpr → Bytecode Compilation

Add method to compile `FfiExpr` to bytecode:

```rust
impl ExpressionChecker {
    /// Compile an FFI default expression to bytecode.
    fn compile_ffi_expr(&mut self, expr: &FfiExpr) -> Option<ExprInfo> {
        match expr {
            FfiExpr::Int(v) => {
                self.bytecode.emit(Instruction::PushInt(*v));
                Some(ExprInfo::rvalue(DataType::simple(primitives::INT32)))
            }
            FfiExpr::UInt(v) => {
                self.bytecode.emit(Instruction::PushUInt(*v));
                Some(ExprInfo::rvalue(DataType::simple(primitives::UINT32)))
            }
            FfiExpr::Float(v) => {
                self.bytecode.emit(Instruction::PushDouble(*v));
                Some(ExprInfo::rvalue(DataType::simple(primitives::DOUBLE)))
            }
            FfiExpr::Bool(v) => {
                self.bytecode.emit(Instruction::PushBool(*v));
                Some(ExprInfo::rvalue(DataType::simple(primitives::BOOL)))
            }
            FfiExpr::String(s) => {
                let idx = self.bytecode.add_string_constant(s.clone());
                self.bytecode.emit(Instruction::PushString(idx));
                Some(ExprInfo::rvalue(DataType::simple(primitives::STRING)))
            }
            FfiExpr::Null => {
                self.bytecode.emit(Instruction::PushNull);
                Some(ExprInfo::rvalue(DataType::simple(primitives::NULL)))
            }
            FfiExpr::EnumValue { enum_name, value_name } => {
                // Look up enum type and value
                let type_hash = self.context.lookup_type(enum_name)?;
                let value = self.context.lookup_enum_value(type_hash, value_name)?;
                self.bytecode.emit(Instruction::PushInt(value));
                Some(ExprInfo::rvalue(DataType::simple(type_hash)))
            }
            FfiExpr::Unary { op, expr } => {
                // Compile operand, then apply unary op
                let operand = self.compile_ffi_expr(expr)?;
                self.compile_unary_op(*op, operand)
            }
            FfiExpr::Binary { left, op, right } => {
                // Compile both operands, then apply binary op
                let left_info = self.compile_ffi_expr(left)?;
                let right_info = self.compile_ffi_expr(right)?;
                self.compile_binary_op(*op, left_info, right_info)
            }
            FfiExpr::Construct { type_name, args } => {
                // Compile constructor call
                // ... handle type lookup and constructor resolution
                todo!("Constructor default args")
            }
            FfiExpr::Ident(name) => {
                // Look up as global constant
                // ... handle constant lookup
                todo!("Identifier default args")
            }
            FfiExpr::ScopedIdent { scope, name } => {
                // Look up scoped constant
                todo!("Scoped identifier default args")
            }
        }
    }
}
```

### 5. Update Call Compilation

Replace the error with actual compilation:

```rust
// In call compilation, when arg_count < param_count:
for i in arg_contexts.len()..func_ref.param_count() {
    if let Some(default_value) = self.context.get_default_value(func_hash, i) {
        let default_info = match default_value {
            DefaultValue::Script(expr) => self.check_expr(expr)?,
            DefaultValue::Ffi(ffi_expr) => self.compile_ffi_expr(ffi_expr)?,
        };

        // Apply implicit conversion if needed
        let param_type = func_ref.param_type(i);
        if let Some(conv) = default_info.data_type.can_convert_to(param_type, self.context) {
            self.emit_conversion(&conv);
        }
    } else {
        // No default - error
        self.error(...);
        return None;
    }
}
```

## Test Cases

```rust
#[test]
fn ffi_function_with_default_int() {
    // Register: void greet(string name, int times = 1)
    let mut builder = FfiRegistryBuilder::new();
    builder.register_function_with_defaults(
        FunctionBuilder::new("greet")
            .with_params(vec![
                Param::new("name", DataType::string()),
                Param::with_default("times", DataType::simple(primitives::INT32)),
            ])
            .build(),
        None,
        vec![(1, FfiExpr::Int(1))],
    );

    // Script: greet("World");  // Should compile to: push "World", push 1, call greet
}

#[test]
fn ffi_function_with_default_string() {
    // Register: void greet(string name = "World")
    // Script: greet();  // Should compile to: push "World", call greet
}

#[test]
fn ffi_function_with_default_enum() {
    // Register: void setColor(Color c = Color::Red)
    // Script: setColor();  // Should compile to: push 0 (Red's value), call setColor
}
```

## Verification

```bash
cargo build --workspace
cargo test --workspace
cargo test -p angelscript-compiler -- ffi_default
```

## Status

**Blocked**: Waiting for Tasks 8-15 (new compiler passes) to be completed.
