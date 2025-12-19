# Task 46b: Class Member Initialization

## Overview

Implement class member initialization in declaration order, as per AngelScript semantics.

## Goals

1. Compile field initializers (`int x = 5;`)
2. Insert initialization code at start of constructors
3. Maintain declaration order
4. Handle both default and custom constructors

## Dependencies

- Task 43b: Assignment Expressions (for field assignments)
- Task 46: Function Compilation (constructor compilation)

## Doc Reference

https://www.angelcode.com/angelscript/sdk/docs/manual/doc_script_class_memberinit.html

## Current State

### Parser Support (Complete)

```rust
// crates/angelscript-parser/src/ast/decl.rs:164
pub struct FieldDecl<'ast> {
    pub visibility: Visibility,
    pub ty: TypeExpr<'ast>,
    pub name: Ident<'ast>,
    pub init: Option<&'ast Expr<'ast>>,  // <- Field initializer
    pub span: Span,
}
```

### Registration Pass (Ignores init)

```rust
// crates/angelscript-compiler/src/passes/registration.rs:364
fn visit_field(&mut self, field: &FieldDecl<'_>, class_hash: TypeHash) {
    // ... resolves type, creates PropertyEntry
    // NOTE: field.init is completely ignored!
}
```

### Compilation Pass (Skips fields)

```rust
// crates/angelscript-compiler/src/passes/compilation.rs:172
ClassMember::Field(_) => {
    // Fields and virtual properties have no bytecode
}
```

## Files to Modify

```
crates/angelscript-compiler/src/passes/
├── compilation.rs    # Compile field initializers into constructors
└── registration.rs   # Store field init expressions for later

crates/angelscript-compiler/src/
└── field_init.rs     # NEW: Field initialization bytecode generation
```

## Detailed Implementation

### 1. Store Field Initializers During Registration

Modify `RegistrationPass` to store field initializer AST references for later compilation.

```rust
// In PendingResolutions or separate structure
pub struct PendingFieldInits {
    /// class_hash -> vec of (field_index, init_expr_span) in declaration order
    pub inits: FxHashMap<TypeHash, Vec<(u16, Span)>>,
}
```

### 2. Compile Field Initializers

Create new module to generate field initialization bytecode:

```rust
// field_init.rs
pub fn compile_field_initializers(
    ctx: &mut CompilationContext<'_>,
    emitter: &mut BytecodeEmitter<'_>,
    class_hash: TypeHash,
    class: &ClassDecl<'_>,
) -> Result<()> {
    for (field_idx, member) in class.members.iter().enumerate() {
        if let ClassMember::Field(field) = member {
            if let Some(init) = &field.init {
                // Compile initializer expression
                let field_type = resolve_field_type(ctx, field)?;
                let mut expr_compiler = ExprCompiler::new(ctx, emitter, Some(class_hash));

                // Get 'this' on stack
                emitter.emit_get_this();

                // Compile value
                expr_compiler.check(init, &field_type)?;

                // Store to field
                emitter.emit_set_field(field_idx as u16);
            }
        }
    }
    Ok(())
}
```

### 3. Inject Into Constructors

Modify constructor compilation to prepend field initialization:

```rust
fn compile_function_body(
    &mut self,
    func_entry: &FunctionEntry,
    body: &Block<'_>,
    owner: Option<TypeHash>,
    span: Span,
) -> Result<BytecodeChunk, CompilationError> {
    let mut compiler = FunctionCompiler::new(...);
    compiler.setup_parameters(span)?;

    // NEW: If this is a constructor, compile field initializers first
    if func_entry.def.is_constructor() {
        if let Some(class_hash) = owner {
            compile_field_initializers(
                self.ctx,
                &mut compiler.emitter,
                class_hash,
                self.find_class_ast(class_hash)?, // Need to look up AST
            )?;
        }
    }

    compiler.compile_body(body)?;
    compiler.verify_returns(span)?;
    Ok(compiler.finish())
}
```

### 4. Handle Default Constructor

For auto-generated default constructors, the field initialization IS the entire body:

```rust
fn generate_default_constructor(class_hash: TypeHash, class: &ClassDecl<'_>) -> BytecodeChunk {
    let mut emitter = BytecodeEmitter::new(...);

    // Compile all field initializers
    compile_field_initializers(...)?;

    // Return void
    emitter.emit_return_void();

    emitter.finish()
}
```

## Bytecode Layout

For a class like:
```angelscript
class Foo {
    int x = 10;
    int y = 20;

    Foo() {
        print("constructor");
    }
}
```

The constructor bytecode should be:
```
; Field initialization (declaration order)
GetThis
PushInt 10
SetField 0           ; x = 10
GetThis
PushInt 20
SetField 1           ; y = 20

; User constructor body
PushString "constructor"
Call print
ReturnVoid
```

## Edge Cases

1. **No initializer**: `int x;` - field left uninitialized (default value)
2. **Complex initializer**: `int x = foo() + 5;` - expression must be compiled
3. **Self-referential**: `int y = x;` - depends on declaration order
4. **Handle fields**: `Foo@ obj = Foo();` - must instantiate
5. **Base class**: base class initializers run first (via super())

## Testing

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn field_initializer_simple() {
        // class A { int x = 5; }
    }

    #[test]
    fn field_initializer_order() {
        // class A { int x = 1; int y = 2; int z = 3; }
        // Must initialize in order: x, y, z
    }

    #[test]
    fn field_initializer_with_constructor() {
        // class A { int x = 5; A() { x = 10; } }
        // Constructor body runs AFTER field init
    }

    #[test]
    fn field_initializer_expression() {
        // class A { int x = calculate(); }
    }

    #[test]
    fn field_initializer_default_constructor() {
        // class A { int x = 5; }  // No explicit ctor
        // Auto-generated ctor must init x
    }
}
```

## Acceptance Criteria

- [ ] Field initializers are compiled
- [ ] Initialization happens in declaration order
- [ ] Custom constructors include field init before body
- [ ] Default constructors initialize all fields
- [ ] Complex initializer expressions work
- [ ] All tests pass

## Notes

- AngelScript initializes in **declaration order**, not alphabetical
- Field init runs BEFORE constructor body
- Base class fields init via base constructor call
- Handle fields require proper ref-counting
