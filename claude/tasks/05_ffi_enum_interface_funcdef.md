# Task 05: Enum, Interface, and Funcdef Registration

**Status:** Not Started
**Depends On:** Task 01, Task 02
**Estimated Scope:** Direct registration with declaration string parsing

---

## Objective

Implement `Module.register_enum()`, `Module.register_interface()`, and `Module.register_funcdef()` for registering enums, interfaces, and function pointer types. All use full AngelScript declaration string parsing - no separate builders needed.

## Files to Create/Modify

- `src/ffi/module.rs` - Add `register_enum`, `register_interface`, `register_funcdef` methods

## API Design

```rust
impl<'app> Module<'app> {
    /// Register an enum with full declaration parsing
    ///
    /// Declaration format: "enum Name { Value1 [= N], Value2, ... }"
    /// Examples:
    ///   - "enum Color { Red = 0, Green = 1, Blue = 2 }"
    ///   - "enum Direction { North, East, South, West }"  // auto-numbered 0,1,2,3
    ///   - "enum Flags { None = 0, Read = 1, Write = 2, Execute = 4 }"
    pub fn register_enum(&mut self, decl: &str) -> Result<(), FfiRegistrationError>;

    /// Register an interface with full declaration parsing
    ///
    /// Declaration format: "interface Name { method1(); method2(); ... }"
    /// Examples:
    ///   - "interface IDrawable { void draw() const; void setVisible(bool); }"
    ///   - "interface ISerializable { string serialize() const; void deserialize(const string &in); }"
    pub fn register_interface(&mut self, decl: &str) -> Result<(), FfiRegistrationError>;

    /// Register a funcdef (function pointer type) with full declaration parsing
    ///
    /// Declaration format: "funcdef ReturnType Name(params)"
    /// Examples:
    ///   - "funcdef void Callback()"
    ///   - "funcdef bool Predicate(int value)"
    ///   - "funcdef void EventHandler(const string &in event, ?&in data)"
    pub fn register_funcdef(&mut self, decl: &str) -> Result<(), FfiRegistrationError>;
}
```

## Usage Examples

### Enums

```rust
// Basic enum with explicit values
module.register_enum("enum Color { Red = 0, Green = 1, Blue = 2 }")?;

// Auto-numbered enum (values: 0, 1, 2, 3)
module.register_enum("enum Direction { North, East, South, West }")?;

// Flags with power-of-2 values
module.register_enum("enum FileFlags { None = 0, Read = 1, Write = 2, Execute = 4, All = 7 }")?;

// Multi-line for readability
module.register_enum("
    enum Status {
        Pending = 0,
        Running = 1,
        Completed = 2,
        Failed = 3,
        Cancelled = 4
    }
")?;
```

### Interfaces

```rust
// Simple interface
module.register_interface("interface IDrawable { void draw() const; void setVisible(bool); }")?;

// Serialization interface
module.register_interface("
    interface ISerializable {
        string serialize() const;
        void deserialize(const string &in data);
    }
")?;

// Complex interface with multiple methods
module.register_interface("
    interface IGameEntity {
        string getName() const;
        void setName(const string &in name);
        Vec3 getPosition() const;
        void setPosition(const Vec3 &in pos);
        void update(float deltaTime);
        void render() const;
    }
")?;
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

**Key Design Decision:** We reuse the existing AST parser infrastructure rather than creating a separate parsing system. All three registration methods leverage the existing parser.

### Parser Analysis

Looking at the existing parser (`src/ast/decl_parser.rs`):

1. **`parse_enum`** - Already ends at `}` without requiring trailing semicolon - **usable as-is**
2. **`parse_interface`** - Already ends at `}` without requiring trailing semicolon - **usable as-is**
3. **`parse_funcdef`** - Requires semicolon at line 2531 - **needs refactoring**

### Refactoring Required

Only `parse_funcdef` needs internal refactoring to separate signature parsing from the semicolon requirement:

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

For **enum** and **interface**, the existing `parse_enum` and `parse_interface` methods can be called directly - they already end at `}`. The FFI code just needs to verify EOF after calling them.

### Parsed Result Types

These wrap existing AST types where possible:

```rust
// Enum declaration - values parsed with existing expression parser
pub struct EnumDecl<'ast> {
    pub name: &'ast Ident<'ast>,
    pub values: Vec<EnumValue<'ast>>,
}

pub struct EnumValue<'ast> {
    pub name: &'ast Ident<'ast>,
    pub value: Option<i64>,  // Parsed from const expr, None = auto-increment
}

// Interface declaration - methods use existing FunctionParam parsing
pub struct InterfaceDecl<'ast> {
    pub name: &'ast Ident<'ast>,
    pub methods: Vec<InterfaceMethod<'ast>>,
}

pub struct InterfaceMethod<'ast> {
    pub name: &'ast Ident<'ast>,
    pub params: Vec<&'ast FunctionParam<'ast>>,  // Reuses existing type
    pub return_type: &'ast TypeExpr<'ast>,       // Reuses existing type
    pub is_const: bool,
}

// Funcdef declaration - fully reuses existing types
pub struct FuncdefDecl<'ast> {
    pub name: &'ast Ident<'ast>,
    pub params: Vec<&'ast FunctionParam<'ast>>,  // Reuses existing type
    pub return_type: &'ast TypeExpr<'ast>,       // Reuses existing type
}
```

### Module's Parsing Methods

```rust
impl<'app> Module<'app> {
    pub fn register_enum(&mut self, decl: &str) -> Result<&mut Self, FfiRegistrationError> {
        let lexer = Lexer::new(decl, "ffi");
        let mut parser = Parser::new(lexer, &self.arena);
        let enum_decl = parser.parse_enum_decl()
            .map_err(|e| FfiRegistrationError::ParseError { ... })?;

        // Store parsed enum with arena-allocated AST nodes
        self.enums.push(enum_decl);
        Ok(self)
    }

    // Similar for register_interface and register_funcdef
}
```

## Internal Storage

Each registration stores parsed AST types:

```rust
pub(crate) struct NativeEnumDef<'ast> {
    pub name: &'ast Ident<'ast>,
    pub values: Vec<(&'ast Ident<'ast>, i64)>,
}

pub(crate) struct NativeInterfaceDef<'ast> {
    pub name: &'ast Ident<'ast>,
    pub methods: Vec<InterfaceMethod<'ast>>,
}

pub(crate) struct NativeFuncdefDef<'ast> {
    pub name: &'ast Ident<'ast>,
    pub params: Vec<&'ast FunctionParam<'ast>>,
    pub return_type: &'ast TypeExpr<'ast>,
}
```

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

1. Enum values without explicit assignment auto-increment from previous value (or 0)
2. Interface methods are signatures only - no implementations
3. Funcdefs create named function pointer types
4. All are stored with arena-allocated AST nodes
5. Namespace is inherited from the Module

## Acceptance Criteria

- [ ] Enums can be registered with full declaration strings
- [ ] Enum values support explicit and auto-incremented values
- [ ] Interfaces can be registered with method signatures
- [ ] Interface method constness is parsed correctly
- [ ] Funcdefs create proper function pointer types
- [ ] Multi-line declarations work correctly
- [ ] Parse errors return descriptive messages
- [ ] All work with the namespace system
