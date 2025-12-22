# Phase 6: Compilation Pass Updates

## Overview

Update the Compilation pass to use the resolved types from Completion. The pass now works with:
- Resolved `DataType` in function signatures
- Hash indexes for bytecode generation
- `QualifiedName`-based lookups via the registry

**Files:**
- `crates/angelscript-compiler/src/passes/compilation.rs` (update)
- `crates/angelscript-compiler/src/type_resolver.rs` (move here for expression type checking)

---

## Key Changes

### TypeResolver Moves to Compilation

During Compilation, `TypeResolver` is used for type checking expressions (not for signature resolution - that's done in Completion):

```rust
// crates/angelscript-compiler/src/type_resolver.rs

use angelscript_core::{DataType, QualifiedName, TypeEntry};
use angelscript_registry::SymbolRegistry;

/// Type resolver for expression type checking during compilation.
///
/// Uses fully resolved registry (hash indexes built).
pub struct TypeResolver<'ctx> {
    ctx: &'ctx CompilationContext,
}

impl<'ctx> TypeResolver<'ctx> {
    pub fn new(ctx: &'ctx CompilationContext) -> Self {
        Self { ctx }
    }

    /// Resolve a type expression from the AST.
    ///
    /// During compilation, all type names should exist (registered in Pass 1,
    /// resolved in Pass 1b). This is for resolving types in expressions like
    /// casts, sizeof, etc.
    pub fn resolve(&self, ty: &Type<'_>) -> Result<DataType, CompilationError> {
        // Parse type name from AST
        let name = self.type_to_string(ty);

        // Look up in registry using current namespace context
        let qualified_name = self.ctx.resolve_type_name(&name)?;

        // Get the type entry
        let entry = self.ctx.get_type(&qualified_name)?;
        let type_hash = entry.type_hash();

        // Build DataType
        let mut dt = DataType::simple(type_hash);

        if entry.as_interface().is_some() {
            dt = dt.with_is_interface(true);
        }
        if entry.as_class().is_some_and(|c| c.is_mixin) {
            dt = dt.with_is_mixin(true);
        }
        if entry.as_enum().is_some() {
            dt = dt.with_is_enum(true);
        }

        Ok(dt)
    }

    /// Resolve a type with all modifiers from a TypeExpr.
    pub fn resolve_type_expr(&self, ty: &TypeExpr<'_>) -> Result<DataType, CompilationError> {
        let mut dt = self.resolve(&ty.ty)?;

        if ty.is_const {
            dt = dt.with_const(true);
        }
        if ty.is_handle {
            dt = dt.with_handle(true);
        }
        if ty.is_handle_to_const {
            dt = dt.with_handle_to_const(true);
        }
        if ty.ref_modifier != ast::RefModifier::None {
            dt = dt.with_ref_modifier(self.convert_ref_modifier(ty.ref_modifier));
        }

        Ok(dt)
    }
}
```

### CompilationContext Updates

```rust
// crates/angelscript-compiler/src/context.rs

impl CompilationContext<'_> {
    /// Resolve a type name in the current namespace context.
    ///
    /// Uses the registry's namespace resolution with current namespace and imports.
    pub fn resolve_type_name(&self, name: &str) -> Result<QualifiedName, CompilationError> {
        self.registry.resolve_type_name(
            name,
            &self.current_namespace,
            &self.imports,
        ).ok_or_else(|| CompilationError::UnknownType {
            name: name.to_string(),
            span: Span::default(),
        })
    }

    /// Get a type by qualified name.
    pub fn get_type(&self, name: &QualifiedName) -> Result<&TypeEntry, CompilationError> {
        self.registry.get(name)
            .or_else(|| self.global_registry.get(name))
            .ok_or_else(|| CompilationError::UnknownType {
                name: name.to_string(),
                span: Span::default(),
            })
    }

    /// Get a type by hash (for bytecode generation).
    ///
    /// Requires hash indexes to be built (after Completion pass).
    pub fn get_type_by_hash(&self, hash: TypeHash) -> Option<&TypeEntry> {
        self.registry.get_by_hash(hash)
            .or_else(|| self.global_registry.get_by_hash(hash))
    }

    /// Get function by hash (for bytecode generation).
    pub fn get_function_by_hash(&self, hash: TypeHash) -> Option<&FunctionEntry> {
        self.registry.get_function_by_hash(hash)
            .or_else(|| self.global_registry.get_function_by_hash(hash))
    }
}
```

---

## Function Body Compilation

Function signatures are already resolved - just compile bodies:

```rust
// crates/angelscript-compiler/src/passes/compilation.rs

impl<'a, 'reg> CompilationPass<'a, 'reg> {
    fn compile_function(&mut self, func: &FunctionDecl<'_>, object_type: Option<&QualifiedName>) {
        let name = func.name.name;
        let qualified_name = self.qualified_name(name);

        // Get the function entry (signature already resolved in Completion)
        let func_entry = match self.get_function(&qualified_name, object_type) {
            Some(entry) => entry,
            None => {
                self.ctx.add_error(CompilationError::Other {
                    message: format!("function not found: {}", qualified_name),
                    span: func.span,
                });
                return;
            }
        };

        // Signature is already resolved
        assert!(func_entry.def.is_resolved(), "Function signature not resolved");

        // Skip abstract methods and native functions
        if func_entry.def.is_abstract() || func_entry.def.is_native {
            return;
        }

        // Get body
        let body = match &func.body {
            Some(body) => body,
            None => return, // Declaration only
        };

        // Set up local scope with parameters
        self.enter_function_scope(&func_entry.def);

        // Compile function body
        self.compile_block(body);

        // Emit return
        self.emit_return(&func_entry.def.return_type);

        // Exit scope
        self.exit_function_scope();
    }

    fn enter_function_scope(&mut self, func: &FunctionDef) {
        self.ctx.enter_scope();

        // Add parameters to scope
        for param in &func.params {
            self.ctx.add_local(&param.name, param.data_type.clone());
        }

        // Add 'this' if method
        if let Some(object_type) = &func.object_type {
            let this_type = self.ctx.get_type(object_type)
                .map(|e| DataType::simple(e.type_hash()).with_handle(true))
                .unwrap_or(DataType::void());
            self.ctx.add_local("this", this_type);
        }
    }

    fn get_function(
        &self,
        name: &QualifiedName,
        object_type: Option<&QualifiedName>,
    ) -> Option<&FunctionEntry> {
        let funcs = self.ctx.registry.get_functions(name)?;

        // If method, find the one with matching object type
        if let Some(obj) = object_type {
            funcs.iter().find(|f| {
                f.def.object_type.as_ref() == Some(obj)
            })
        } else {
            // Global function - should be unique by signature
            funcs.first()
        }
    }
}
```

---

## Method Lookup in Type Checking

```rust
impl<'a, 'reg> CompilationPass<'a, 'reg> {
    /// Look up a method on a type.
    fn lookup_method(
        &self,
        type_name: &QualifiedName,
        method_name: &str,
    ) -> Vec<&FunctionEntry> {
        // Get the class entry
        let class = match self.ctx.get_type(type_name).and_then(|e| e.as_class()) {
            Some(c) => c,
            None => return vec![],
        };

        // Use vtable for method lookup (includes inherited methods)
        let slots = class.vtable.slots_for_name(method_name);

        slots.iter()
            .filter_map(|&slot| class.vtable.method_at(slot))
            .filter_map(|hash| self.ctx.get_function_by_hash(hash))
            .collect()
    }

    /// Look up callable methods by name (for overload resolution).
    fn find_callable_methods(
        &self,
        type_name: &QualifiedName,
        method_name: &str,
    ) -> Vec<&FunctionEntry> {
        let class = match self.ctx.get_type(type_name).and_then(|e| e.as_class()) {
            Some(c) => c,
            None => return vec![],
        };

        class.find_callable_methods(method_name)
            .into_iter()
            .filter_map(|hash| self.ctx.get_function_by_hash(hash))
            .collect()
    }
}
```

---

## Interface Method Dispatch

```rust
impl<'a, 'reg> CompilationPass<'a, 'reg> {
    /// Compile an interface method call.
    fn compile_interface_call(
        &mut self,
        receiver: &Expr<'_>,
        method_name: &str,
        args: &[Expr<'_>],
    ) {
        // Type check receiver
        let receiver_type = self.type_of(receiver);

        // Get interface entry
        let iface_name = self.ctx.get_type_name(receiver_type.type_hash)?;
        let iface = self.ctx.get_type(&iface_name)
            .and_then(|e| e.as_interface())?;

        // Look up method in itable
        let slots = iface.method_slots_by_name(method_name);

        // Compile receiver and args
        self.compile_expr(receiver);
        for arg in args {
            self.compile_expr(arg);
        }

        // Emit interface call bytecode
        // Uses itable slot for dispatch
        let slot = slots[0]; // TODO: overload resolution
        self.emit_interface_call(receiver_type.type_hash, slot);
    }
}
```

---

## Bytecode Generation

TypeHash is now available for all types (computed in Completion):

```rust
impl<'a, 'reg> CompilationPass<'a, 'reg> {
    /// Emit a type instantiation.
    fn emit_new(&mut self, type_name: &QualifiedName) {
        let entry = self.ctx.get_type(type_name).unwrap();
        let type_hash = entry.type_hash();

        // Emit bytecode with type hash
        self.bytecode.push(Op::New(type_hash));
    }

    /// Emit a function call.
    fn emit_call(&mut self, func: &FunctionEntry) {
        let func_hash = func.func_hash();

        // Emit bytecode with function hash
        self.bytecode.push(Op::Call(func_hash));
    }

    /// Emit a virtual method call.
    fn emit_virtual_call(&mut self, class_hash: TypeHash, slot: u16) {
        self.bytecode.push(Op::VirtualCall { class_hash, slot });
    }

    /// Emit an interface method call.
    fn emit_interface_call(&mut self, iface_hash: TypeHash, slot: u16) {
        self.bytecode.push(Op::InterfaceCall { iface_hash, slot });
    }
}
```

---

## What Stays the Same

1. **Expression type checking** - Still uses TypeResolver (moved here)
2. **Overload resolution** - Still matches args to params
3. **Bytecode emission** - Same instructions, uses TypeHash from entries
4. **Scope management** - Same local variable tracking
5. **Control flow** - Same if/while/for compilation

---

## Migration Notes

### Changes from Current Code

1. **Type lookups**: Use `QualifiedName` instead of `TypeHash` where possible
2. **Hash lookups**: Use `get_by_hash()` only for bytecode generation (after Completion)
3. **TypeResolver**: Only used for expression types, not signature resolution
4. **Function lookup**: Use `QualifiedName` to find function, then hash for bytecode

### Assumptions

- All types are resolved before Compilation starts
- Hash indexes are built after Completion
- Function signatures have resolved `params` and `return_type`

---

## Dependencies

- Phase 5: Completion builds hash indexes
- All types have `QualifiedName` and can compute `type_hash()`
- All functions have resolved signatures

This is the final phase - after Compilation, bytecode is ready for execution.
