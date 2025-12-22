# Phase 7: Compilation Pass Updates

## Overview

Update the Compilation pass to use the fully-resolved registry from Completion. The core compilation logic stays the same - we just update the type lookup APIs.

**Files:**
- `crates/angelscript-compiler/src/passes/compilation.rs` (update)
- `crates/angelscript-compiler/src/type_resolver.rs` (update - now used only for expression types)
- `crates/angelscript-compiler/src/context.rs` (update)

---

## Key Changes

### Registry is Now Read-Only

The compilation pass receives a fully-resolved registry. No mutations needed.

```rust
pub struct CompilationPass<'reg, 'global> {
    /// Unit registry (read-only, fully resolved).
    registry: &'reg SymbolRegistry,
    /// Global registry (read-only, FFI types).
    global_registry: &'global SymbolRegistry,
    // ... rest unchanged
}
```

### Type Lookup by QualifiedName

Where we previously had `TypeHash` lookups, we can now use `QualifiedName`:

```rust
impl<'reg, 'global> CompilationPass<'reg, 'global> {
    /// Get a type by name.
    fn get_type(&self, name: &QualifiedName) -> Option<&TypeEntry> {
        self.registry.get_type_by_name(name)
            .or_else(|| self.global_registry.get_type_by_name(name))
    }

    /// Get a class by name.
    fn get_class(&self, name: &QualifiedName) -> Option<&ClassEntry> {
        self.get_type(name).and_then(|e| e.as_class())
    }

    /// Get an interface by name.
    fn get_interface(&self, name: &QualifiedName) -> Option<&InterfaceEntry> {
        self.get_type(name).and_then(|e| e.as_interface())
    }

    /// Resolve a type name in current context.
    fn resolve_type_name(&self, name: &str) -> Option<QualifiedName> {
        self.registry.resolve_type_name(
            name,
            &self.current_namespace,
            &self.imports,
        ).or_else(|| {
            self.global_registry.resolve_type_name(
                name,
                &self.current_namespace,
                &self.imports,
            )
        })
    }
}
```

### Hash Lookup for Bytecode

When emitting bytecode, we still need `TypeHash` values. These are now available from the entries:

```rust
impl<'reg, 'global> CompilationPass<'reg, 'global> {
    /// Emit a type instantiation.
    fn emit_new(&mut self, type_name: &QualifiedName) {
        let entry = self.get_type(type_name).expect("type should exist");
        let type_hash = entry.type_hash();
        self.bytecode.push(Op::New(type_hash));
    }

    /// Emit a function call.
    fn emit_call(&mut self, func: &FunctionEntry) {
        let func_hash = func.func_hash();
        self.bytecode.push(Op::Call(func_hash));
    }

    /// Get type by hash (for bytecode validation).
    fn get_type_by_hash(&self, hash: TypeHash) -> Option<&TypeEntry> {
        self.registry.get_by_hash(hash)
            .or_else(|| self.global_registry.get_by_hash(hash))
    }
}
```

---

## TypeResolver for Expressions

The `TypeResolver` is now only used during compilation for resolving types in expressions (casts, sizeof, etc.), not for function signatures (which are resolved in Completion).

```rust
// crates/angelscript-compiler/src/type_resolver.rs

/// Type resolver for expression type checking during compilation.
///
/// All function signatures and type declarations are already resolved.
/// This is only used for resolving type expressions in code bodies.
pub struct TypeResolver<'ctx> {
    ctx: &'ctx CompilationContext,
}

impl<'ctx> TypeResolver<'ctx> {
    pub fn new(ctx: &'ctx CompilationContext) -> Self {
        Self { ctx }
    }

    /// Resolve a type expression from the AST.
    pub fn resolve(&self, ty: &TypeExpr<'_>) -> Result<DataType, CompilationError> {
        let name = self.type_to_string(&ty.ty);

        // Resolve using current namespace context
        let qualified_name = self.ctx.resolve_type_name(&name)
            .ok_or_else(|| CompilationError::UnknownType {
                name: name.clone(),
                span: ty.span,
            })?;

        // Get the type entry
        let entry = self.ctx.get_type(&qualified_name)
            .ok_or_else(|| CompilationError::UnknownType {
                name: qualified_name.to_string(),
                span: ty.span,
            })?;

        let type_hash = entry.type_hash();

        // Build DataType with flags
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

        // Apply modifiers from TypeExpr
        if ty.is_const {
            dt = dt.with_const(true);
        }
        if ty.is_handle {
            dt = dt.with_handle(true);
        }
        if ty.is_handle_to_const {
            dt = dt.with_handle_to_const(true);
        }
        if ty.ref_modifier != RefModifier::None {
            dt = dt.with_ref_modifier(convert_ref_modifier(ty.ref_modifier));
        }

        Ok(dt)
    }
}
```

---

## CompilationContext Updates

```rust
// crates/angelscript-compiler/src/context.rs

impl<'reg> CompilationContext<'reg> {
    /// Resolve a type name in the current namespace context.
    pub fn resolve_type_name(&self, name: &str) -> Option<QualifiedName> {
        self.unit_registry.resolve_type_name(
            name,
            &self.current_namespace,
            &self.imports,
        ).or_else(|| {
            self.global_registry.resolve_type_name(
                name,
                &self.current_namespace,
                &self.imports,
            )
        })
    }

    /// Get a type by qualified name.
    pub fn get_type(&self, name: &QualifiedName) -> Option<&TypeEntry> {
        self.unit_registry.get_type_by_name(name)
            .or_else(|| self.global_registry.get_type_by_name(name))
    }

    /// Get a type by hash (for bytecode operations).
    pub fn get_type_by_hash(&self, hash: TypeHash) -> Option<&TypeEntry> {
        self.unit_registry.get_by_hash(hash)
            .or_else(|| self.global_registry.get_by_hash(hash))
    }

    /// Get functions by name (for overload resolution).
    pub fn get_functions(&self, name: &QualifiedName) -> Vec<&FunctionEntry> {
        let mut result = self.unit_registry.get_functions_by_name(name);
        result.extend(self.global_registry.get_functions_by_name(name));
        result
    }

    /// Get a class by name.
    pub fn get_class(&self, name: &QualifiedName) -> Option<&ClassEntry> {
        self.get_type(name).and_then(|e| e.as_class())
    }

    /// Get an interface by name.
    pub fn get_interface(&self, name: &QualifiedName) -> Option<&InterfaceEntry> {
        self.get_type(name).and_then(|e| e.as_interface())
    }
}
```

---

## Method Lookup Changes

```rust
impl<'reg, 'global> CompilationPass<'reg, 'global> {
    /// Look up methods on a type by name.
    fn lookup_methods(
        &self,
        type_name: &QualifiedName,
        method_name: &str,
    ) -> Vec<&FunctionEntry> {
        let class = match self.get_class(type_name) {
            Some(c) => c,
            None => return vec![],
        };

        // Get all method hashes with this name (from vtable)
        class.find_callable_methods(method_name)
            .into_iter()
            .filter_map(|hash| self.get_function_by_hash(hash))
            .collect()
    }

    /// Get function entry by hash.
    fn get_function_by_hash(&self, hash: TypeHash) -> Option<&FunctionEntry> {
        self.registry.get_function_by_hash(hash)
            .or_else(|| self.global_registry.get_function_by_hash(hash))
    }
}
```

---

## What Stays the Same

Most of the compilation pass logic is unchanged:

1. **Expression compilation** - Same AST walking, bytecode emission
2. **Statement compilation** - Same control flow, variable handling
3. **Overload resolution** - Same algorithm, just different lookup APIs
4. **Type checking** - Same rules, just uses `TypeResolver` for expressions only
5. **Bytecode emission** - Same opcodes, uses `TypeHash` from entries

---

## Usage Example

```rust
pub fn compile_unit(
    ast: &Script<'_>,
    global_registry: &SymbolRegistry,
) -> Result<BytecodeModule, Vec<CompilationError>> {
    // Pass 1: Registration
    let registration_result = RegistrationPass::new(unit_id).run(ast);
    if registration_result.has_errors() {
        return Err(registration_result.errors);
    }

    // Pass 2: Completion
    let mut unit_registry = SymbolRegistry::new();
    let completion_result = CompletionPass::new(&mut unit_registry, global_registry)
        .run(registration_result);
    if completion_result.has_errors() {
        return Err(completion_result.errors);
    }

    // Pass 3: Compilation (uses resolved registry)
    let compilation_result = CompilationPass::new(&unit_registry, global_registry)
        .run(ast);

    compilation_result
}
```

---

## Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn compile_source(source: &str) -> Result<BytecodeModule, Vec<CompilationError>> {
        let ast = parse_script(source).unwrap();
        let unit_id = UnitId::new(0);

        // Pass 1
        let reg_result = RegistrationPass::new(unit_id).run(&ast);
        if reg_result.has_errors() {
            return Err(reg_result.errors);
        }

        // Pass 2
        let global_registry = SymbolRegistry::new();
        let mut unit_registry = SymbolRegistry::new();
        let comp_result = CompletionPass::new(&mut unit_registry, &global_registry)
            .run(reg_result);
        if comp_result.has_errors() {
            return Err(comp_result.errors);
        }

        // Pass 3
        CompilationPass::new(&unit_registry, &global_registry).run(&ast)
    }

    #[test]
    fn compile_forward_reference() {
        let result = compile_source(r#"
            interface IDamageable {
                void attack(Player@ p);
            }
            class Player : IDamageable {
                void attack(Player@ p) {
                    // Method body compiles successfully
                }
            }
        "#);

        assert!(result.is_ok());
    }

    #[test]
    fn compile_circular_methods() {
        let result = compile_source(r#"
            class Foo {
                void doSomething(Bar@ b) {
                    b.doSomething(this);
                }
            }
            class Bar {
                void doSomething(Foo@ f) {
                    f.doSomething(this);
                }
            }
        "#);

        assert!(result.is_ok());
    }

    #[test]
    fn compile_namespace_types() {
        let result = compile_source(r#"
            namespace Game {
                class Entity {}
                class Player {
                    Entity@ target;
                    void setTarget(Entity@ e) {
                        target = e;
                    }
                }
            }
        "#);

        assert!(result.is_ok());
    }
}
```

---

## Migration Checklist

1. [ ] Update `CompilationPass` to use name-based lookup
2. [ ] Update `CompilationContext` with new lookup methods
3. [ ] Update `TypeResolver` to work with resolved registry
4. [ ] Remove any direct `TypeHash` computation during compilation
5. [ ] Update tests to use new 3-pass flow
6. [ ] Verify all existing tests still pass

---

## Summary

The compilation pass changes are minimal:
- Use `QualifiedName` for type lookup
- Get `TypeHash` from entry (already computed)
- `TypeResolver` only used for expression types, not signatures
- Registry is read-only after completion

This completes the QualifiedName-Based Registry Architecture.
