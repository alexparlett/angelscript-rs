# Task 46c: Import Declarations

## Overview

Implement import declaration compilation for cross-module function imports.

## Goals

1. Register imported functions in symbol registry
2. Mark functions as imported with source module
3. Enable runtime linking of imports

## Dependencies

- Task 38: Registration Pass

## Doc Reference

https://www.angelcode.com/angelscript/sdk/docs/manual/doc_global_import.html

## Syntax

```angelscript
import void playSound(const string &in filename) from "audio";
import int calculateDamage(int base, float modifier) from "combat";
```

## Current State

### Parser Support (Complete)

```rust
// crates/angelscript-parser/src/ast/decl.rs:391
pub struct ImportDecl<'ast> {
    pub return_type: ReturnType<'ast>,
    pub name: Ident<'ast>,
    pub params: &'ast [FunctionParam<'ast>],
    pub attrs: FuncAttr,
    pub module: String,  // Source module name
    pub span: Span,
}
```

### Registration Pass (Skipped)

```rust
// crates/angelscript-compiler/src/passes/registration.rs
// In compile_item():
Item::Import(_) => { /* Not handled */ }
```

### Compilation Pass (Skipped)

```rust
// crates/angelscript-compiler/src/passes/compilation.rs
// In compile_item():
Item::Import(_) => { /* Not handled */ }
```

## Files to Modify

```
crates/angelscript-core/src/entries/function.rs
    Add FunctionSource::Import variant

crates/angelscript-compiler/src/passes/registration.rs
    Handle Item::Import in visit_item()
```

## Detailed Implementation

### 1. Add Import Source Type

```rust
// In crates/angelscript-core/src/entries/function.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionSource {
    /// Native function implemented in Rust via FFI
    Native {
        native_fn: NativeFn,
    },
    /// Script function with bytecode
    Script {
        bytecode_offset: u32,
    },
    /// External function (linked at load time)
    External,
    /// Imported from another module (NEW)
    Import {
        module: String,
    },
}

impl FunctionEntry {
    pub fn is_import(&self) -> bool {
        matches!(self.source, FunctionSource::Import { .. })
    }

    pub fn import_module(&self) -> Option<&str> {
        match &self.source {
            FunctionSource::Import { module } => Some(module),
            _ => None,
        }
    }
}
```

### 2. Register Import in Registration Pass

```rust
// In crates/angelscript-compiler/src/passes/registration.rs

fn visit_item(&mut self, item: &Item<'_>) {
    match item {
        // ... existing cases ...

        Item::Import(import) => self.register_import(import),

        // ...
    }
}

fn register_import(&mut self, import: &ImportDecl<'_>) {
    // Resolve return type
    let mut resolver = TypeResolver::new(self.ctx);
    let return_type = match resolver.resolve(&import.return_type.ty) {
        Ok(dt) => dt,
        Err(e) => {
            self.ctx.add_error(e);
            return;
        }
    };

    // Resolve parameter types
    let params: Vec<Param> = import
        .params
        .iter()
        .filter_map(|p| {
            resolver.resolve(&p.ty.ty).ok().map(|dt| {
                Param::new(&p.name.map(|n| n.name).unwrap_or(""), dt)
            })
        })
        .collect();

    let param_hashes: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();

    // Create qualified name
    let name = import.name.name.to_string();
    let qualified_name = self.qualified_name(&name);

    // Create function hash
    let func_hash = TypeHash::from_function(&qualified_name, &param_hashes);

    // Create function definition
    let def = FunctionDef::new(
        func_hash,
        qualified_name.clone(),
        vec![], // No template params for imports
        params,
        return_type,
        None, // No default arg count
        convert_func_attrs(import.attrs),
        false, // Not a system function
        Visibility::Public,
    );

    // Create function entry with Import source
    let entry = FunctionEntry {
        def,
        source: FunctionSource::Import {
            module: import.module.clone(),
        },
        type_source: TypeSource::script(import.span),
    };

    // Register the function
    if let Err(e) = self.ctx.unit_registry_mut().register_function(entry) {
        self.ctx.add_error(CompilationError::Other {
            message: format!("failed to register import '{}': {}", qualified_name, e),
            span: import.span,
        });
    }

    self.functions_registered += 1;
}
```

### 3. No Compilation Needed

Imported functions have no bytecode - they're linked at runtime. The compilation pass can continue to skip them.

### 4. Runtime Linking (Future Work)

When loading a module, the runtime must:

1. Find all imported functions (`FunctionSource::Import`)
2. Look up the source module by name
3. Find the matching function in source module
4. Update the import to point to actual function

```rust
// Future: In runtime/vm module
fn link_imports(module: &mut Module, modules: &ModuleRegistry) -> Result<()> {
    for func in module.functions() {
        if let FunctionSource::Import { module: source_name } = &func.source {
            let source_module = modules.get(source_name)?;
            let target_func = source_module.find_function(&func.def.name, &func.def.params)?;
            // Link import to target...
        }
    }
    Ok(())
}
```

## Edge Cases

1. **Circular imports**: A imports from B, B imports from A
2. **Missing module**: Import from non-existent module
3. **Missing function**: Function not found in source module
4. **Signature mismatch**: Import signature differs from actual function
5. **Namespace handling**: `import ns::func from "module"`

## Testing

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn register_simple_import() {
        // import void foo() from "bar";
    }

    #[test]
    fn register_import_with_params() {
        // import int add(int a, int b) from "math";
    }

    #[test]
    fn register_import_with_ref_params() {
        // import void process(const string &in s) from "util";
    }

    #[test]
    fn import_is_callable() {
        // import void foo() from "bar";
        // void main() { foo(); }
    }

    #[test]
    fn import_in_namespace() {
        // namespace ns { import void foo() from "bar"; }
    }
}
```

## Acceptance Criteria

- [ ] Import declarations are parsed correctly (already done)
- [ ] Imports are registered in symbol registry
- [ ] Import source tracks module name
- [ ] Calling imported functions compiles (generates Call opcode)
- [ ] All tests pass

## Notes

- Imports are registered like any other function
- Only difference is `FunctionSource::Import` instead of `Script`
- Runtime linking is out of scope for this task
- No bytecode generated for import declarations
