# Task 05: Enum, Interface, and Funcdef Registration

**Status:** Completed
**Depends On:** Task 01, Task 02
**Estimated Scope:** Builder pattern for enum/interface, declaration string for funcdef

---

## Objective

Implement `Module.register_enum()`, `Module.register_interface()`, and `Module.register_funcdef()` for registering enums, interfaces, and function pointer types. Enum and interface use builder patterns (like `ClassBuilder`), while funcdef uses declaration string parsing.

## Files to Create/Modify

- `src/ffi/module.rs` - Add `register_enum`, `register_interface`, `register_funcdef` methods
- `src/ffi/enum_builder.rs` - `EnumBuilder` implementation
- `src/ffi/interface_builder.rs` - `InterfaceBuilder` implementation

## Design Decision: AST Primitive Reuse

- **Enums**: Don't use AST types - builder provides simple string names and i64 values
- **Interfaces**: Use AST primitives (`Ident`, `FunctionParam`, `ReturnType`) for method signatures
- **Funcdefs**: Use AST primitives for the full signature, parsed via `parse_ffi_funcdef`

## API Design

```rust
impl<'app> Module<'app> {
    /// Register an enum type, returning a builder
    ///
    /// Example:
    ///   module.register_enum("Color")
    ///       .value("Red", 0)?
    ///       .value("Green", 1)?
    ///       .build()?;
    pub fn register_enum(&mut self, name: &str) -> EnumBuilder<'_, 'app>;

    /// Register an interface type, returning a builder
    ///
    /// Example:
    ///   module.register_interface("IDrawable")
    ///       .method("void draw() const")?
    ///       .build()?;
    pub fn register_interface(&mut self, name: &str) -> InterfaceBuilder<'_, 'app>;

    /// Register a funcdef (function pointer type) with declaration string
    ///
    /// Declaration format: "funcdef ReturnType Name(params)"
    /// Examples:
    ///   - "funcdef void Callback()"
    ///   - "funcdef bool Predicate(int value)"
    ///   - "funcdef void EventHandler(const string &in event, ?&in data)"
    pub fn register_funcdef(&mut self, decl: &str) -> Result<&mut Self, FfiRegistrationError>;
}
```

## EnumBuilder

```rust
pub struct EnumBuilder<'m, 'app> {
    module: &'m mut Module<'app>,
    name: String,
    values: Vec<(String, i64)>,
    next_value: i64,
}

impl<'m, 'app> EnumBuilder<'m, 'app> {
    /// Add an enum value with explicit integer value
    pub fn value(mut self, name: &str, value: i64) -> Result<Self, FfiRegistrationError> {
        self.values.push((name.to_string(), value));
        self.next_value = value + 1;
        Ok(self)
    }

    /// Add an enum value with auto-incremented value
    pub fn auto_value(mut self, name: &str) -> Result<Self, FfiRegistrationError> {
        self.values.push((name.to_string(), self.next_value));
        self.next_value += 1;
        Ok(self)
    }

    /// Finish building and register the enum
    pub fn build(self) -> Result<(), FfiRegistrationError>;
}
```

## InterfaceBuilder

```rust
pub struct InterfaceBuilder<'m, 'app> {
    module: &'m mut Module<'app>,
    name: String,
    methods: Vec<NativeMethodDef<'m>>,
}

impl<'m, 'app> InterfaceBuilder<'m, 'app> {
    /// Add an interface method using declaration string
    ///
    /// Declaration format: "ReturnType name(params) [const]"
    /// Examples:
    ///   - "void draw() const"
    ///   - "string serialize() const"
    ///   - "void setName(const string &in name)"
    pub fn method(mut self, decl: &str) -> Result<Self, FfiRegistrationError>;

    /// Finish building and register the interface
    pub fn build(self) -> Result<(), FfiRegistrationError>;
}
```

## Usage Examples

### Enums

```rust
// Basic enum with explicit values
module.register_enum("Color")
    .value("Red", 0)?
    .value("Green", 1)?
    .value("Blue", 2)?
    .build()?;

// Auto-numbered enum (values: 0, 1, 2, 3)
module.register_enum("Direction")
    .auto_value("North")?
    .auto_value("East")?
    .auto_value("South")?
    .auto_value("West")?
    .build()?;

// Flags with power-of-2 values
module.register_enum("FileFlags")
    .value("None", 0)?
    .value("Read", 1)?
    .value("Write", 2)?
    .value("Execute", 4)?
    .value("All", 7)?
    .build()?;

// Mixed explicit and auto values
module.register_enum("Status")
    .value("Pending", 0)?
    .auto_value("Running")?      // 1
    .auto_value("Completed")?    // 2
    .value("Failed", -1)?        // explicit
    .value("Cancelled", -2)?     // explicit
    .build()?;
```

### Interfaces

```rust
// Simple interface
module.register_interface("IDrawable")
    .method("void draw() const")?
    .method("void setVisible(bool visible)")?
    .build()?;

// Serialization interface
module.register_interface("ISerializable")
    .method("string serialize() const")?
    .method("void deserialize(const string &in data)")?
    .build()?;

// Complex interface with multiple methods
module.register_interface("IGameEntity")
    .method("string getName() const")?
    .method("void setName(const string &in name)")?
    .method("Vec3 getPosition() const")?
    .method("void setPosition(const Vec3 &in pos)")?
    .method("void update(float deltaTime)")?
    .method("void render() const")?
    .build()?;
```

### Funcdefs

```rust
// Simple callback
module.register_funcdef("funcdef void Callback()")?;

// Predicate function
module.register_funcdef("funcdef bool Predicate(int value)")?;

// Event handler with variable type
module.register_funcdef("funcdef void EventHandler(const string &in event, ?&in data)")?;

// Comparator function
module.register_funcdef("funcdef int Comparator(const T &in a, const T &in b)")?;

// Factory function
module.register_funcdef("funcdef Entity@ EntityFactory(const string &in name)")?;
```

## AST Reuse Strategy

**Key Design Decision:** We reuse the existing AST parser infrastructure for parsing method signatures (in `InterfaceBuilder`) and funcdef declarations.

### Parser Analysis

Looking at the existing parser (`src/ast/decl_parser.rs`):

1. **`parse_funcdef`** - Requires semicolon at line 2531 - **needs refactoring**
2. **`parse_function_or_global_var`** - Requires semicolon - **needs refactoring** (for method signatures)
3. **`parse_enum`** - NOT needed for FFI (using builder pattern)
4. **`parse_interface`** - NOT needed for FFI (using builder pattern)

### Refactoring Required

Only `parse_funcdef` needs refactoring for FFI funcdef registration:

```rust
impl<'ast> Parser<'ast> {
    // Existing method - still requires semicolon for script parsing
    pub fn parse_funcdef(&mut self, modifiers: DeclModifiers) -> Result<Item<'ast>, ParseError> {
        let (decl, _) = self.parse_funcdef_inner(modifiers)?;
        self.expect(TokenKind::Semicolon)?;  // Required for scripts
        Ok(Item::Funcdef(decl))
    }

    // New internal method - no semicolon
    fn parse_funcdef_inner(&mut self, modifiers: DeclModifiers) -> Result<(FuncdefDecl<'ast>, Span), ParseError> {
        let start_span = self.expect(TokenKind::FuncDef)?.span;
        let return_type = self.parse_return_type()?;
        let name = self.parse_ident()?;
        let template_params = if self.check(TokenKind::Less) {
            self.parse_template_param_names()?
        } else {
            self.arena.alloc_slice_copy(&[])
        };
        let params = self.parse_function_params()?;
        let end_span = self.peek_previous().span;

        Ok((FuncdefDecl {
            modifiers,
            return_type,
            name,
            template_params,
            params,
            span: start_span.merge(end_span),
        }, end_span))
    }

    // FFI entry point - accepts EOF
    pub fn parse_ffi_funcdef(&mut self) -> Result<FuncdefDecl<'ast>, ParseError> {
        let (decl, _) = self.parse_funcdef_inner(DeclModifiers::new())?;
        self.expect_eof()?;
        Ok(decl)
    }
}
```

For **InterfaceBuilder.method()**, we reuse the function signature parsing from Task 03 (`parse_ffi_function_signature`).

## Internal Storage

FFI-specific container types that compose AST primitives (see ffi_plan.md "FFI Storage Types"). IDs are assigned at registration time using global atomic counters (`TypeId::next()`).

```rust
/// Enum - simple strings and values (no AST parsing needed)
/// Builder provides resolved values, not parsed AST
pub(crate) struct NativeEnumDef {
    pub id: TypeId,                 // Assigned via TypeId::next() at registration
    pub name: String,
    pub values: Vec<(String, i64)>,
}

/// Interface - uses AST primitives for method signatures
pub(crate) struct NativeInterfaceDef<'ast> {
    pub id: TypeId,                 // Assigned via TypeId::next() at registration
    pub name: String,
    pub methods: Vec<NativeInterfaceMethod<'ast>>,
}

/// Interface method signature - no implementation (scripts implement these)
/// NO FunctionId - these are abstract signatures, not callable functions
pub(crate) struct NativeInterfaceMethod<'ast> {
    pub name: Ident<'ast>,                    // AST primitive
    pub params: &'ast [FunctionParam<'ast>],  // AST primitive slice
    pub return_type: ReturnType<'ast>,        // AST primitive
    pub is_const: bool,
}

/// Funcdef (function pointer type) - uses AST primitives
pub(crate) struct NativeFuncdefDef<'ast> {
    pub id: TypeId,                           // Assigned via TypeId::next() at registration
    pub name: Ident<'ast>,                    // AST primitive
    pub params: &'ast [FunctionParam<'ast>],  // AST primitive slice
    pub return_type: ReturnType<'ast>,        // AST primitive
}
```

**Key points:**
- `NativeEnumDef` doesn't use AST types - the builder provides simple strings and i64 values
- `NativeEnumDef`, `NativeInterfaceDef`, and `NativeFuncdefDef` all get a `TypeId` at registration
- `NativeInterfaceMethod` does NOT get a `FunctionId` - these are abstract method signatures that scripts implement
- `NativeInterfaceDef` and `NativeFuncdefDef` use AST primitives for parsed signatures
- All `'ast` lifetime types are arena-allocated in Module's `Bump` arena

## Error Handling

```rust
pub enum FfiRegistrationError {
    /// Failed to parse declaration string
    ParseError { decl: String, error: String },
    /// Duplicate enum value name
    DuplicateEnumValue { enum_name: String, value_name: String },
    /// Duplicate type name
    DuplicateName { name: String },
    /// Invalid enum value (e.g., non-integer)
    InvalidEnumValue { value: String, error: String },
    // ...
}
```

## Implementation Notes

1. EnumBuilder uses simple string names and i64 values (no AST parsing needed)
2. InterfaceBuilder parses method signatures using existing FFI function signature parser
3. Funcdefs parse full declaration strings using `parse_ffi_funcdef`
4. Interface methods are signatures only - no implementations (scripts implement them)
5. Namespace is inherited from the Module
6. Enum values can be explicit or auto-incremented

## Acceptance Criteria

- [x] `register_enum` returns `EnumBuilder`
- [x] `EnumBuilder.value()` adds explicit enum values
- [x] `EnumBuilder.auto_value()` adds auto-incremented values
- [x] `register_interface` returns `InterfaceBuilder`
- [x] `InterfaceBuilder.method()` parses method declaration strings
- [x] Interface method constness is parsed correctly
- [x] `register_funcdef` parses full funcdef declaration strings
- [x] Funcdefs support template parameters
- [x] Parse errors return descriptive messages
- [x] All work with the namespace system
