# Task 32: String Factory Configuration

## Overview

Add `StringFactory` trait and configuration to `Context` to specify how string literals are converted to the target string type. This allows users to configure custom string implementations (interned strings, OsString, ASCII-optimized, etc.).

## Goals

1. Define `StringFactory` trait with raw bytes input
2. Add `string_factory` field to `Context`
3. Add `Context::set_string_factory()` API
4. Implement default `ScriptStringFactory`
5. Update `with_default_modules()` to set default factory
6. Add `NoStringFactory` error variant

## Dependencies

- Task 31: Compiler Foundation (for `Constant::StringData` storage)
- Main crate's `Context` type in `src/context.rs`
- `angelscript-modules` crate for `ScriptString`

## Background

String literals in AngelScript-Rust use a factory pattern (similar to C++ `asIStringFactory`) to support custom string implementations. Raw byte data is stored in the constant pool, and the factory creates the actual string value at runtime.

**Why raw bytes?**
- No UTF-8 assumption - factory interprets bytes however it wants
- Supports non-UTF8 escape sequences (`\xFF`, etc.)
- Enables OsString, ASCII-optimized strings, interned strings, etc.

## Detailed Implementation

### StringFactory Trait (src/string_factory.rs)

```rust
use angelscript_core::TypeHash;

/// Trait for creating string values from raw byte data.
/// Similar to C++ AngelScript's asIStringFactory.
///
/// Implement this trait to use custom string types for string literals.
pub trait StringFactory: Send + Sync {
    /// Create a string value from raw bytes.
    /// Called by the VM when loading string constants.
    /// The factory interprets the bytes however it wants (UTF-8, ASCII, etc.)
    fn create(&self, data: &[u8]) -> Box<dyn std::any::Any + Send + Sync>;

    /// The type hash of the string type this factory produces.
    /// Used by the compiler for type checking string literals.
    fn type_hash(&self) -> TypeHash;
}
```

### ScriptStringFactory (angelscript-modules/src/string.rs)

```rust
use crate::ScriptString;
use angelscript::StringFactory;
use angelscript_core::Any as CoreAny;
use angelscript_core::TypeHash;

/// Default string factory that creates ScriptString values.
pub struct ScriptStringFactory;

impl StringFactory for ScriptStringFactory {
    fn create(&self, data: &[u8]) -> Box<dyn std::any::Any + Send + Sync> {
        // Interpret as UTF-8, lossy conversion for invalid sequences
        let s = String::from_utf8_lossy(data);
        Box::new(ScriptString::from(s.as_ref()))
    }

    fn type_hash(&self) -> TypeHash {
        <ScriptString as CoreAny>::type_hash()
    }
}
```

### Context Extension (src/context.rs)

```rust
use crate::StringFactory;

impl Context {
    /// The string factory for creating string literal values.
    /// If None, string literals will produce a compile error.
    string_factory: Option<Box<dyn StringFactory>>,

    /// Set a custom string factory.
    ///
    /// # Example
    /// ```
    /// ctx.set_string_factory(Box::new(ScriptStringFactory));
    /// ```
    pub fn set_string_factory(&mut self, factory: Box<dyn StringFactory>) {
        self.string_factory = Some(factory);
    }

    /// Get the string factory (for compiler/VM use).
    pub fn string_factory(&self) -> Option<&dyn StringFactory> {
        self.string_factory.as_deref()
    }
}
```

### Update with_default_modules() (src/context.rs)

```rust
impl Context {
    pub fn with_default_modules() -> Self {
        let mut ctx = Self::new();

        // Install modules...
        ctx.install_module::<StringModule>();
        ctx.install_module::<MathModule>();
        ctx.install_module::<ArrayModule>();
        // ...

        // Set default string factory
        ctx.set_string_factory(Box::new(ScriptStringFactory));

        ctx
    }
}
```

### Constant Pool Update (Task 31 - already planned)

```rust
pub enum Constant {
    Int(i64),
    Uint(u64),
    Float32(f32),
    Float64(f64),
    /// Raw string literal bytes. NOT a script string type.
    /// The VM passes this to Context::string_factory().create() to produce
    /// the actual string value.
    StringData(Vec<u8>),
    TypeHash(TypeHash),
}
```

### Compiler Error (crates/angelscript-compiler/src/error.rs)

```rust
#[derive(Debug, Error)]
pub enum CompileError {
    // ... existing variants ...

    #[error("no string factory configured - call Context::set_string_factory() or use with_default_modules()")]
    NoStringFactory { span: Span },
}
```

## Usage in Compiler (Task 41 will implement)

```rust
fn compile_string_literal(raw_bytes: Vec<u8>, span: Span) -> Result<ExprInfo> {
    let factory = ctx.string_factory()
        .ok_or(CompileError::NoStringFactory { span })?;

    // Store raw bytes in constant pool
    let string_idx = constants.add(Constant::StringData(raw_bytes));

    // Emit opcode to load string (VM will call factory.create())
    emitter.emit_load_string(string_idx);

    Ok(ExprInfo::rvalue(DataType::simple(factory.type_hash())))
}
```

## VM Usage

```rust
fn execute_load_string(index: u32, constants: &ConstantPool, factory: &dyn StringFactory) -> Value {
    let raw = match constants.get(index) {
        Some(Constant::StringData(bytes)) => bytes.as_slice(),
        _ => panic!("expected string constant"),
    };

    let boxed = factory.create(raw);
    Value::from_boxed(boxed, factory.type_hash())
}
```

## Custom Factory Examples

### OsString Factory
```rust
pub struct OsStringFactory;

impl StringFactory for OsStringFactory {
    fn create(&self, data: &[u8]) -> Box<dyn std::any::Any + Send + Sync> {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            Box::new(std::ffi::OsString::from(std::ffi::OsStr::from_bytes(data)))
        }
        #[cfg(windows)]
        {
            let s = String::from_utf8_lossy(data);
            Box::new(std::ffi::OsString::from(s.as_ref()))
        }
    }

    fn type_hash(&self) -> TypeHash {
        TypeHash::from_name("os_string")
    }
}
```

### Interned String Factory
```rust
pub struct InternedStringFactory {
    interner: Arc<Mutex<HashSet<Arc<[u8]>>>>,
}

impl StringFactory for InternedStringFactory {
    fn create(&self, data: &[u8]) -> Box<dyn std::any::Any + Send + Sync> {
        let mut interner = self.interner.lock().unwrap();
        if let Some(existing) = interner.get(data) {
            Box::new(existing.clone())
        } else {
            let arc: Arc<[u8]> = data.into();
            interner.insert(arc.clone());
            Box::new(arc)
        }
    }

    fn type_hash(&self) -> TypeHash {
        TypeHash::from_name("interned_string")
    }
}
```

## Usage Examples (Script Perspective)

```angelscript
string s = "hello";           // Uses default factory -> string type
MyString m = "hello";         // Type mismatch: expected MyString, got string
MyString m = MyString("hello"); // Explicit construction works
auto x = "hello";             // Infers to string type (from factory)
```

## Testing

```rust
#[test]
fn string_factory_not_set() {
    let ctx = Context::new(); // No modules
    assert!(ctx.string_factory().is_none());
}

#[test]
fn string_factory_set_by_default_modules() {
    let ctx = Context::with_default_modules();
    let factory = ctx.string_factory().expect("should have factory");
    assert_eq!(factory.type_hash(), <ScriptString as Any>::type_hash());
}

#[test]
fn custom_string_factory() {
    let mut ctx = Context::new();
    ctx.set_string_factory(Box::new(ScriptStringFactory));

    let factory = ctx.string_factory().unwrap();
    let value = factory.create(b"hello");
    assert_eq!(factory.type_hash(), <ScriptString as Any>::type_hash());
}

#[test]
fn factory_handles_non_utf8() {
    let ctx = Context::with_default_modules();
    let factory = ctx.string_factory().unwrap();

    // Invalid UTF-8 sequence
    let data = vec![0xFF, 0xFE, 0x00];
    let value = factory.create(&data);
    // Should not panic - lossy conversion handles invalid UTF-8
}
```

## Acceptance Criteria

- [x] `StringFactory` trait defined in angelscript-core
- [x] `ScriptStringFactory` implemented in angelscript-modules
- [x] `Context::set_string_factory()` method added
- [x] `Context::string_factory()` getter added
- [x] `with_default_modules()` sets default factory
- [x] `NoStringFactory` error variant added to compiler
- [x] Factory handles non-UTF8 bytes gracefully
- [x] All tests pass

## Next Phase

Task 33: Compilation Context - wraps TypeRegistry with compilation state
