# Task 08g: List Behaviors Infrastructure

**Status:** Not Started
**Parent:** Task 08 - Built-in Modules
**Depends On:** 08d (modules structure), ClassBuilder exists

---

## Objective

Add support for list construction behaviors (`list_construct`, `list_factory`) to the FFI system. This enables initialization list syntax:

```angelscript
array<int> a = {1, 2, 3};
dictionary@ d = {{"key1", 1}, {"key2", 2}};
```

## Files to Modify

- `src/ffi/types.rs` - Add ListBehavior, ListPattern, extend Behaviors
- `src/ffi/class_builder.rs` - Add list_construct(), list_factory() methods
- `src/ffi/native_fn.rs` - Add ListBuffer type
- `src/ffi/mod.rs` - Export new types

## Background

AngelScript supports initialization lists for constructing objects. Two behaviors handle this:

- `asBEHAVE_LIST_CONSTRUCT` - For value types (constructs in-place)
- `asBEHAVE_LIST_FACTORY` - For reference types (returns handle)

The list pattern describes what the initialization list contains:
- `{repeat T}` - Zero or more elements of type T
- `{int, string}` - Fixed sequence: int followed by string
- `{repeat {K, V}}` - Repeated pairs (for dictionary)

## Implementation

### 1. Types (src/ffi/types.rs)

```rust
/// Pattern describing expected initialization list format.
#[derive(Debug, Clone)]
pub enum ListPattern<'ast> {
    /// Zero or more elements of type T: `{repeat T}`
    Repeat(TypeExpr<'ast>),

    /// Fixed sequence of types: `{int, string}`
    Fixed(Vec<TypeExpr<'ast>>),

    /// Repeated tuples: `{repeat {K, V}}`
    RepeatTuple(Vec<TypeExpr<'ast>>),
}

/// List construction behavior with its pattern.
#[derive(Debug, Clone)]
pub struct ListBehavior<'ast> {
    /// Native function to call
    pub native_fn: NativeFn,
    /// Expected list pattern
    pub pattern: ListPattern<'ast>,
}

// Extend existing Behaviors struct
pub struct Behaviors<'ast> {
    pub factory: Vec<BehaviorDef<'ast>>,
    pub addref: Option<NativeFn>,
    pub release: Option<NativeFn>,
    pub construct: Vec<BehaviorDef<'ast>>,
    pub destruct: Option<NativeFn>,
    pub copy_construct: Option<NativeFn>,
    pub assign: Option<NativeFn>,

    // NEW: List behaviors
    pub list_construct: Option<ListBehavior<'ast>>,  // For value types
    pub list_factory: Option<ListBehavior<'ast>>,    // For reference types
}
```

### 2. ListBuffer (src/ffi/native_fn.rs)

```rust
/// Buffer containing initialization list data.
///
/// Provides typed access to elements passed via `{1, 2, 3}` syntax.
pub struct ListBuffer<'a> {
    /// Raw element data
    elements: &'a [VmSlot],
    /// Element type (for type checking)
    element_type: TypeId,
}

impl<'a> ListBuffer<'a> {
    /// Create from raw slot slice
    pub fn new(elements: &'a [VmSlot], element_type: TypeId) -> Self {
        Self { elements, element_type }
    }

    /// Number of elements in the list
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Get element at index
    pub fn get(&self, index: usize) -> Option<&VmSlot> {
        self.elements.get(index)
    }

    /// Iterate over elements
    pub fn iter(&self) -> impl Iterator<Item = &VmSlot> {
        self.elements.iter()
    }

    /// Get element type
    pub fn element_type(&self) -> TypeId {
        self.element_type
    }
}

/// Buffer for tuple-based list (e.g., dictionary initialization).
pub struct TupleListBuffer<'a> {
    /// Tuples stored as flattened array
    data: &'a [VmSlot],
    /// Number of elements per tuple
    tuple_size: usize,
    /// Types of each tuple element
    element_types: Vec<TypeId>,
}

impl<'a> TupleListBuffer<'a> {
    pub fn new(data: &'a [VmSlot], tuple_size: usize, element_types: Vec<TypeId>) -> Self {
        Self { data, tuple_size, element_types }
    }

    /// Number of tuples
    pub fn len(&self) -> usize {
        self.data.len() / self.tuple_size
    }

    /// Get tuple at index as slice
    pub fn get_tuple(&self, index: usize) -> Option<&[VmSlot]> {
        let start = index * self.tuple_size;
        let end = start + self.tuple_size;
        if end <= self.data.len() {
            Some(&self.data[start..end])
        } else {
            None
        }
    }

    /// Iterate over tuples
    pub fn iter(&self) -> impl Iterator<Item = &[VmSlot]> {
        self.data.chunks_exact(self.tuple_size)
    }
}
```

### 3. ClassBuilder Methods (src/ffi/class_builder.rs)

```rust
impl<'m, 'app, T: NativeType> ClassBuilder<'m, 'app, T> {
    /// Register list construction behavior for value types.
    ///
    /// # Example
    ///
    /// ```rust
    /// module.register_type::<MyStruct>("MyStruct")
    ///     .value_type()
    ///     .list_construct("void f(int &in list) {repeat int}", my_list_construct)?
    ///     .build()?;
    /// ```
    ///
    /// The declaration includes the list pattern after the signature:
    /// - `{repeat T}` - Zero or more elements of type T
    /// - `{T, U}` - Fixed sequence of types
    /// - `{repeat {T, U}}` - Repeated tuples
    pub fn list_construct<F>(mut self, decl: &str, f: F) -> Result<Self, FfiRegistrationError>
    where
        F: Fn(&mut CallContext) -> Result<(), NativeError> + Send + Sync + 'static,
    {
        let (signature, pattern) = self.parse_list_declaration(decl)?;

        // Verify this is a value type
        if !self.is_value_type() {
            return Err(FfiRegistrationError::InvalidBehavior(
                "list_construct is only valid for value types".into()
            ));
        }

        self.behaviors.list_construct = Some(ListBehavior {
            native_fn: NativeFn::from_raw(f),
            pattern,
        });

        Ok(self)
    }

    /// Register list factory behavior for reference types.
    ///
    /// # Example
    ///
    /// ```rust
    /// module.register_type::<ScriptArray>("array<class T>")
    ///     .reference_type()
    ///     .list_factory("array<T>@ f(int &in list) {repeat T}", array_list_factory)?
    ///     .build()?;
    /// ```
    pub fn list_factory<F>(mut self, decl: &str, f: F) -> Result<Self, FfiRegistrationError>
    where
        F: Fn(&mut CallContext) -> Result<(), NativeError> + Send + Sync + 'static,
    {
        let (signature, pattern) = self.parse_list_declaration(decl)?;

        // Verify this is a reference type
        if !self.is_reference_type() {
            return Err(FfiRegistrationError::InvalidBehavior(
                "list_factory is only valid for reference types".into()
            ));
        }

        self.behaviors.list_factory = Some(ListBehavior {
            native_fn: NativeFn::from_raw(f),
            pattern,
        });

        Ok(self)
    }

    /// Parse declaration with list pattern.
    ///
    /// Format: "signature {pattern}"
    /// Examples:
    /// - "array<T>@ f(int &in list) {repeat T}"
    /// - "void f(int &in list) {int, string}"
    /// - "dictionary<K,V>@ f(int &in list) {repeat {K, V}}"
    fn parse_list_declaration(&self, decl: &str) -> Result<(String, ListPattern<'static>), FfiRegistrationError> {
        // Find the pattern part
        let brace_start = decl.find('{').ok_or_else(|| {
            FfiRegistrationError::ParseError("List declaration must include pattern in braces".into())
        })?;

        let signature = decl[..brace_start].trim().to_string();
        let pattern_str = decl[brace_start..].trim();

        let pattern = self.parse_list_pattern(pattern_str)?;

        Ok((signature, pattern))
    }

    /// Parse list pattern from string.
    fn parse_list_pattern(&self, pattern: &str) -> Result<ListPattern<'static>, FfiRegistrationError> {
        let pattern = pattern.trim();

        // Must start with { and end with }
        if !pattern.starts_with('{') || !pattern.ends_with('}') {
            return Err(FfiRegistrationError::ParseError(
                "List pattern must be enclosed in braces".into()
            ));
        }

        let inner = &pattern[1..pattern.len()-1].trim();

        if inner.starts_with("repeat ") {
            let rest = &inner[7..].trim();

            // Check for nested tuple: {repeat {K, V}}
            if rest.starts_with('{') && rest.ends_with('}') {
                let tuple_inner = &rest[1..rest.len()-1];
                let types = self.parse_type_list(tuple_inner)?;
                Ok(ListPattern::RepeatTuple(types))
            } else {
                // Simple repeat: {repeat T}
                let type_expr = self.parse_type_expr(rest)?;
                Ok(ListPattern::Repeat(type_expr))
            }
        } else {
            // Fixed sequence: {int, string}
            let types = self.parse_type_list(inner)?;
            Ok(ListPattern::Fixed(types))
        }
    }

    fn parse_type_list(&self, s: &str) -> Result<Vec<TypeExpr<'static>>, FfiRegistrationError> {
        s.split(',')
            .map(|t| self.parse_type_expr(t.trim()))
            .collect()
    }

    fn parse_type_expr(&self, s: &str) -> Result<TypeExpr<'static>, FfiRegistrationError> {
        // Use existing parser infrastructure to parse type
        // This may need to use the module's arena for allocation
        todo!("Parse type expression from string")
    }
}
```

### 4. CallContext Extensions (src/ffi/native_fn.rs)

```rust
impl<'vm> CallContext<'vm> {
    /// Get list buffer from argument.
    ///
    /// Used by list_construct/list_factory native functions.
    pub fn arg_list(&self, index: usize) -> Result<ListBuffer<'_>, NativeError> {
        // The list is passed as a special internal type
        // Implementation depends on how the VM passes list data
        todo!("Extract list buffer from argument")
    }

    /// Get tuple list buffer from argument.
    pub fn arg_tuple_list(&self, index: usize, tuple_size: usize) -> Result<TupleListBuffer<'_>, NativeError> {
        todo!("Extract tuple list buffer from argument")
    }
}
```

## Usage Examples

### Array List Factory

```rust
fn array_list_factory(ctx: &mut CallContext) -> Result<(), NativeError> {
    let list = ctx.arg_list(0)?;
    let element_type = ctx.template_type(0)?; // T from array<T>

    let mut arr = ScriptArray::new(element_type);
    for slot in list.iter() {
        arr.push(slot.clone());
    }

    ctx.set_return_handle(arr)?;
    Ok(())
}

// Registration
module.register_type::<ScriptArray>("array<class T>")
    .reference_type()
    .list_factory("array<T>@ f(int &in list) {repeat T}", array_list_factory)?
    .build()?;
```

### Dictionary List Factory

```rust
fn dict_list_factory(ctx: &mut CallContext) -> Result<(), NativeError> {
    let list = ctx.arg_tuple_list(0, 2)?; // Pairs of {K, V}
    let key_type = ctx.template_type(0)?;   // K
    let value_type = ctx.template_type(1)?; // V

    let mut dict = ScriptDict::new(key_type, value_type);
    for pair in list.iter() {
        let key = ScriptKey::from_slot(&pair[0])?;
        let value = pair[1].clone();
        dict.insert(key, value);
    }

    ctx.set_return_handle(dict)?;
    Ok(())
}

// Registration
module.register_type::<ScriptDict>("dictionary<class K, class V>")
    .reference_type()
    .list_factory("dictionary<K,V>@ f(int &in list) {repeat {K, V}}", dict_list_factory)?
    .build()?;
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repeat_pattern() {
        let pattern = parse_list_pattern("{repeat int}").unwrap();
        assert!(matches!(pattern, ListPattern::Repeat(_)));
    }

    #[test]
    fn test_parse_fixed_pattern() {
        let pattern = parse_list_pattern("{int, string}").unwrap();
        assert!(matches!(pattern, ListPattern::Fixed(types) if types.len() == 2));
    }

    #[test]
    fn test_parse_repeat_tuple_pattern() {
        let pattern = parse_list_pattern("{repeat {string, int}}").unwrap();
        assert!(matches!(pattern, ListPattern::RepeatTuple(types) if types.len() == 2));
    }

    #[test]
    fn test_list_buffer() {
        let elements = vec![VmSlot::Int(1), VmSlot::Int(2), VmSlot::Int(3)];
        let buffer = ListBuffer::new(&elements, TypeId::INT32);

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.get(0), Some(&VmSlot::Int(1)));
    }

    #[test]
    fn test_tuple_list_buffer() {
        let data = vec![
            VmSlot::String("a".into()), VmSlot::Int(1),
            VmSlot::String("b".into()), VmSlot::Int(2),
        ];
        let buffer = TupleListBuffer::new(&data, 2, vec![TypeId::STRING_TYPE, TypeId::INT32]);

        assert_eq!(buffer.len(), 2);
        let tuple = buffer.get_tuple(0).unwrap();
        assert_eq!(tuple[0], VmSlot::String("a".into()));
        assert_eq!(tuple[1], VmSlot::Int(1));
    }
}
```

## Acceptance Criteria

- [ ] `ListPattern` enum added to types.rs
- [ ] `ListBehavior` struct added to types.rs
- [ ] `Behaviors` struct extended with `list_construct` and `list_factory`
- [ ] `ListBuffer` and `TupleListBuffer` added to native_fn.rs
- [ ] `ClassBuilder::list_construct()` method added
- [ ] `ClassBuilder::list_factory()` method added
- [ ] Pattern parsing implemented
- [ ] `CallContext::arg_list()` and `arg_tuple_list()` added
- [ ] Unit tests pass
- [ ] `cargo build --lib` succeeds
