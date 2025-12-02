# Task 04: Class Builder

**Status:** Not Started
**Depends On:** Task 01, Task 02, Task 03
**Estimated Scope:** Type registration API with declaration string parsing

---

## Objective

Implement `ClassBuilder` for registering native types (value types, reference types, and templates) with constructors, methods, properties, operators, and behaviors. All registration methods use declaration string parsing.

## Files to Create/Modify

- `src/ffi/class.rs` - ClassBuilder implementation

## Design Decision: AST Primitive Reuse

ClassBuilder builds `NativeTypeDef` which uses FFI-specific container types that compose AST primitives. Methods, properties, and operators store parsed signatures using `Ident`, `FunctionParam`, `TypeExpr`, and `ReturnType` from the AST.

## Key Types

```rust
pub struct ClassBuilder<'m, 'app, T: NativeType> {
    module: &'m mut Module<'app>,
    name: String,
    template_params: Option<&'m [Ident<'m>]>,  // Parsed from "array<class T>"
    type_kind: TypeKind,
    // All these use AST primitives via FFI storage types
    constructors: Vec<NativeMethodDef<'m>>,
    factories: Vec<NativeMethodDef<'m>>,
    methods: Vec<NativeMethodDef<'m>>,
    properties: Vec<NativePropertyDef<'m>>,
    operators: Vec<NativeMethodDef<'m>>,
    behaviors: Behaviors,
    template_callback: Option<Box<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>>,
    _marker: PhantomData<T>,
}

// FFI storage types using AST primitives (see ffi_plan.md):
// IDs are assigned at registration time using global atomic counters

/// Type definition - uses AST primitives for template params
pub struct NativeTypeDef<'ast> {
    pub id: TypeId,                                   // Assigned via TypeId::next() at registration
    pub name: String,
    pub template_params: Option<&'ast [Ident<'ast>]>, // Parsed from "array<class T>"
    pub type_kind: TypeKind,
    pub constructors: Vec<NativeMethodDef<'ast>>,
    pub factories: Vec<NativeMethodDef<'ast>>,
    pub methods: Vec<NativeMethodDef<'ast>>,
    pub properties: Vec<NativePropertyDef<'ast>>,
    pub operators: Vec<NativeMethodDef<'ast>>,
    pub behaviors: Behaviors,
    pub template_callback: Option<Box<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>>,
}

/// Method/constructor/factory/operator - uses AST primitives
pub struct NativeMethodDef<'ast> {
    pub id: FunctionId,                   // Assigned via FunctionId::next() at registration
    pub name: Ident<'ast>,
    pub params: &'ast [FunctionParam<'ast>],
    pub return_type: ReturnType<'ast>,
    pub is_const: bool,
    pub native_fn: NativeFn,
}

/// Property - uses AST primitives
pub struct NativePropertyDef<'ast> {
    pub name: Ident<'ast>,
    pub type_expr: &'ast TypeExpr<'ast>,
    pub is_const: bool,
    pub getter: NativeFn,
    pub setter: Option<NativeFn>,
}

pub enum TypeKind {
    Value { size: usize, align: usize, is_pod: bool },
    Reference { kind: ReferenceKind },
}

pub enum ReferenceKind {
    Standard,      // Full handle support with AddRef/Release
    Scoped,        // RAII-style, no handles (asOBJ_SCOPED)
    SingleRef,     // App-controlled lifetime (asOBJ_NOHANDLE)
    GenericHandle, // Type-erased container (asOBJ_ASHANDLE)
}

pub struct Behaviors {
    pub addref: Option<Box<dyn Fn(*const ()) + Send + Sync>>,
    pub release: Option<Box<dyn Fn(*const ()) + Send + Sync>>,
    pub destruct: Option<Box<dyn Fn(*mut ()) + Send + Sync>>,
}
```

## ClassBuilder API

```rust
impl<'m, 'app, T: NativeType> ClassBuilder<'m, 'app, T> {
    /// Mark as value type (default) - stack allocated, copied on assignment
    pub fn value_type(mut self) -> Self;

    /// Mark as reference type - heap allocated, handle semantics
    pub fn reference_type(mut self) -> Self;

    /// Register template validation callback (for template types)
    pub fn template_callback<F>(mut self, f: F) -> Result<Self, FfiRegistrationError>
    where F: Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync + 'static;

    /// Register a constructor (value types)
    /// Declaration: "void f(params)" e.g. "void f()", "void f(float x, float y)"
    pub fn constructor<F, Args>(mut self, decl: &str, f: F) -> Result<Self, FfiRegistrationError>
    where F: IntoNativeFn<Args, T>;

    /// Register a factory (reference types)
    /// Declaration: "T@ f(params)" e.g. "T@ f()", "T@ f(const string &in)"
    pub fn factory<F, Args>(mut self, decl: &str, f: F) -> Result<Self, FfiRegistrationError>
    where F: IntoNativeFn<Args, T>;

    /// Register AddRef behavior (reference types)
    pub fn addref<F>(mut self, f: F) -> Self
    where F: Fn(&T) + Send + Sync + 'static;

    /// Register Release behavior (reference types)
    pub fn release<F>(mut self, f: F) -> Self
    where F: Fn(&T) + Send + Sync + 'static;

    /// Register destructor (value types)
    pub fn destructor<F>(mut self, f: F) -> Self
    where F: Fn(&mut T) + Send + Sync + 'static;

    /// Register a method
    /// Declaration: "ReturnType name(params) [const]"
    pub fn method<F, Args, Ret>(mut self, decl: &str, f: F) -> Result<Self, FfiRegistrationError>
    where F: IntoNativeFn<(&T, Args), Ret>;

    /// Register a method with raw CallContext
    pub fn method_raw<F>(mut self, decl: &str, f: F) -> Result<Self, FfiRegistrationError>
    where F: NativeCallable + Send + Sync + 'static;

    /// Register a read-only property
    /// Declaration: "Type name" e.g. "float x", "const string name"
    pub fn property_get<V, F>(mut self, decl: &str, getter: F) -> Result<Self, FfiRegistrationError>
    where
        V: ToScript,
        F: Fn(&T) -> V + Send + Sync + 'static;

    /// Register a read-write property
    /// Declaration: "Type name"
    pub fn property<V, G, S>(mut self, decl: &str, getter: G, setter: S) -> Result<Self, FfiRegistrationError>
    where
        V: ToScript + FromScript,
        G: Fn(&T) -> V + Send + Sync + 'static,
        S: Fn(&mut T, V) + Send + Sync + 'static;

    /// Register an operator
    /// Declaration: "ReturnType opName(params)" e.g. "Vec3 opAdd(const Vec3 &in)"
    pub fn operator<F, Args, Ret>(mut self, decl: &str, f: F) -> Result<Self, FfiRegistrationError>
    where F: IntoNativeFn<(&T, Args), Ret>;

    /// Finish building and register the type
    pub fn build(self) -> Result<(), FfiRegistrationError>;
}
```

## Template Type Registration

Templates are registered using `register_type` with `<class T>` syntax in the type name. The parser extracts template parameter names and recognizes them in method signatures:

```rust
// Register type with template parameters
module.register_type::<ScriptArray>("array<class T>")
    .reference_type()
    .template_callback(|info| TemplateValidation::valid())?
    // T is recognized in signatures
    .factory("array<T>@ f()", || ScriptArray::new())?
    .method("void insertLast(const T &in)", array_insert_last)?
    .method("uint length() const", ScriptArray::len)?
    .operator("T& opIndex(uint)", array_index)?
    .build()?;

// Multiple template parameters
module.register_type::<ScriptDict>("dictionary<class K, class V>")
    .reference_type()
    .template_callback(|info| {
        if is_hashable(&info.sub_types[0]) {
            TemplateValidation::valid()
        } else {
            TemplateValidation::invalid("Key must be hashable")
        }
    })?
    .method("void set(const K &in, const V &in)", dict_set)?
    .method("V& opIndex(const K &in)", dict_index)?
    .build()?;
```

## TemplateInstanceInfo and TemplateValidation

```rust
/// Information about a template instantiation for validation callback
pub struct TemplateInstanceInfo {
    /// The template name (e.g., "array")
    pub template_name: String,
    /// The type arguments (e.g., [int] for array<int>)
    pub sub_types: Vec<DataType>,
}

/// Result of template validation callback
pub struct TemplateValidation {
    /// Is this instantiation valid?
    pub is_valid: bool,
    /// Error message if invalid
    pub error: Option<String>,
    /// Should this instance use garbage collection?
    pub needs_gc: bool,
}

impl TemplateValidation {
    pub fn valid() -> Self;
    pub fn invalid(msg: &str) -> Self;
    pub fn with_gc() -> Self;  // Valid and needs garbage collection
}
```

## Usage Examples

**Value Type:**
```rust
module.register_type::<Vec3>("Vec3")
    .value_type()
    .constructor("void f()", || Vec3::default())?
    .constructor("void f(float x, float y, float z)", Vec3::new)?
    .method("float length() const", |v: &Vec3| v.length())?
    .method("void normalize()", |v: &mut Vec3| v.normalize())?
    .property("float x", |v| v.x, |v, x| v.x = x)?
    .property("float y", |v| v.y, |v, y| v.y = y)?
    .property("float z", |v| v.z, |v, z| v.z = z)?
    .property_get("float lengthSq", |v| v.length_squared())?
    .operator("Vec3 opAdd(const Vec3 &in)", |a, b| *a + *b)?
    .operator("Vec3 opSub(const Vec3 &in)", |a, b| *a - *b)?
    .operator("Vec3 opMul(float)", |a, s| *a * s)?
    .operator("bool opEquals(const Vec3 &in)", |a, b| a == b)?
    .build()?;
```

**Reference Type:**
```rust
module.register_type::<Entity>("Entity")
    .reference_type()
    .factory("Entity@ f()", || Entity::new())?
    .factory("Entity@ f(const string &in name)", Entity::with_name)?
    .addref(Entity::add_ref)
    .release(Entity::release)
    .method("string getName() const", |e| e.name.clone())?
    .method("void setName(const string &in)", |e, name| e.name = name)?
    .method("Vec3 getPosition() const", |e| e.position)?
    .method("void setPosition(const Vec3 &in)", |e, pos| e.position = pos)?
    .build()?;
```

**Scoped Reference Type:**
```rust
module.register_type::<FileHandle>("File")
    .reference_type()  // Could add .scoped() variant
    .factory("File@ f(const string &in path)", File::open)?
    .destructor(|f| f.close())  // Called on scope exit
    .method("string readLine()", File::read_line)?
    .method("void writeLine(const string &in)", File::write_line)?
    .build()?;
```

## AST Reuse Strategy

**Key Design Decision:** We reuse the existing AST parser infrastructure rather than creating a separate parsing system.

### Type Declaration Parsing

When `register_type` is called, the type name is parsed to extract:
- Base type name
- Template parameters (if any)

```rust
// Uses existing parser infrastructure
fn parse_type_decl(&self, decl: &str) -> Result<TypeDecl<'_>, FfiRegistrationError> {
    let lexer = Lexer::new(decl, "ffi");
    let mut parser = Parser::new(lexer, &self.arena);
    parser.parse_type_decl()
        .map_err(|e| FfiRegistrationError::ParseError { ... })
}

// "array<class T>" -> TypeDecl { name: "array", template_params: Some(["T"]) }
// "Vec3" -> TypeDecl { name: "Vec3", template_params: None }
```

### Method Declaration Parsing with Template Context

For template types, the ClassBuilder tracks template parameter names and passes them to the parser:

```rust
impl<'m, 'app, T: NativeType> ClassBuilder<'m, 'app, T> {
    fn parse_method_decl(&self, decl: &str) -> Result<FunctionSignature<'_>, FfiRegistrationError> {
        let lexer = Lexer::new(decl, "ffi");
        let mut parser = Parser::new(lexer, &self.module.arena);

        // Pass template params so parser recognizes T, K, V as type placeholders
        if let Some(ref params) = self.template_params {
            parser.set_template_context(params);
        }

        parser.parse_function_signature()
            .map_err(|e| FfiRegistrationError::ParseError { ... })
    }
}

// With template_params = ["T"]:
// "void insertLast(const T &in)" -> T recognized as SubType(0)
// "T& opIndex(uint)" -> return type is reference to SubType(0)
```

### Parser Refactoring

The existing `Parser` needs refactoring to support fragment parsing without semicolons.

**Key constraint:** Normal script parsing must not regress - semicolons are still required where the language expects them.

**Required changes:**
1. **Extract reusable internal methods** - Separate signature parsing logic from statement-level handling
2. **New `parse_ffi_*` entry points** - `parse_ffi_function_signature()`, `parse_ffi_property_decl()`, `parse_ffi_type_decl()` that accept EOF
3. **Template context** - `set_template_context(&[&str])` to recognize type parameter names (T, K, V)
4. **All output uses existing AST types** - `TypeExpr`, `Ident`, `FunctionParam`

```rust
impl<'src, 'ast> Parser<'src, 'ast> {
    /// Set template parameter names so they're recognized as type placeholders
    pub fn set_template_context(&mut self, params: &[&str]) {
        self.template_params = params.iter().map(|s| s.to_string()).collect();
    }

    /// When parsing types, check if identifier is a template param
    fn parse_type_expr(&mut self) -> Result<&'ast TypeExpr<'ast>, ParseError> {
        // ... existing logic ...
        // If identifier matches a template param, create SubType reference
        if let Some(idx) = self.template_params.iter().position(|p| p == name) {
            return Ok(self.arena.alloc(TypeExpr::SubType(idx)));
        }
        // ... rest of type parsing ...
    }

    // FFI entry points - accept EOF, no semicolon required
    pub fn parse_ffi_function_signature(&mut self) -> Result<FunctionSignature<'ast>, ParseError>;
    pub fn parse_ffi_property_decl(&mut self) -> Result<PropertyDecl<'ast>, ParseError>;
    pub fn parse_ffi_type_decl(&mut self) -> Result<TypeDecl<'ast>, ParseError>;
}
```

## Implementation Notes

1. Value types: size/alignment inferred from `size_of::<T>()` and `align_of::<T>()`
2. Reference types require factory, addref, release behaviors (or scoped/single-ref variant)
3. Methods can be const (suffix) or mutable
4. Properties have getter and optional setter
5. Operators use AngelScript operator naming (opAdd, opSub, opMul, opEquals, etc.)
6. Template params are parsed from type declaration and tracked for method signature parsing

## Acceptance Criteria

- [ ] Value types can be registered with constructors
- [ ] Reference types can be registered with factory/addref/release
- [ ] Template types work with `<class T>` syntax
- [ ] Template validation callbacks work
- [ ] Methods work with declaration string parsing
- [ ] Const vs mutable methods distinguished correctly
- [ ] Properties with getters and setters work
- [ ] Operators can be registered with declaration strings
- [ ] Template parameters (T, K, V) recognized in method signatures
- [ ] All methods return `Result` with parse errors
