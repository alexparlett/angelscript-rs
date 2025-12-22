# Phase 3: Registration Result

## Overview

Define `RegistrationResult` as the output of Pass 1. This struct collects all unresolved entries and is passed to Pass 2 (Completion) for resolution and registry population.

**Files:**
- `crates/angelscript-compiler/src/passes/registration_result.rs` (new)
- `crates/angelscript-compiler/src/passes/mod.rs` (update)

---

## RegistrationResult

```rust
// crates/angelscript-compiler/src/passes/registration_result.rs

use angelscript_core::{
    CompilationError, QualifiedName, UnitId,
    UnresolvedClass, UnresolvedEnum, UnresolvedFuncdef,
    UnresolvedFunction, UnresolvedGlobal, UnresolvedInterface, UnresolvedMixin,
};

/// Output of Pass 1 (Registration).
///
/// Contains all type and function declarations collected from the AST,
/// with types still unresolved. Pass 2 (Completion) transforms this into
/// resolved entries and populates the registry.
#[derive(Debug, Default)]
pub struct RegistrationResult {
    /// Unit ID this result is for.
    pub unit_id: UnitId,

    /// Unresolved class declarations.
    pub classes: Vec<UnresolvedClass>,

    /// Unresolved mixin declarations.
    pub mixins: Vec<UnresolvedMixin>,

    /// Unresolved interface declarations.
    pub interfaces: Vec<UnresolvedInterface>,

    /// Unresolved funcdef declarations.
    pub funcdefs: Vec<UnresolvedFuncdef>,

    /// Unresolved enum declarations.
    pub enums: Vec<UnresolvedEnum>,

    /// Unresolved global function declarations.
    pub functions: Vec<UnresolvedFunction>,

    /// Unresolved global variable declarations.
    pub globals: Vec<UnresolvedGlobal>,

    /// Errors encountered during registration.
    /// These are typically syntax-level issues or duplicate declarations.
    pub errors: Vec<CompilationError>,
}

impl RegistrationResult {
    /// Create a new empty registration result.
    pub fn new(unit_id: UnitId) -> Self {
        Self {
            unit_id,
            ..Default::default()
        }
    }

    /// Add a class declaration.
    pub fn add_class(&mut self, class: UnresolvedClass) {
        self.classes.push(class);
    }

    /// Add a mixin declaration.
    pub fn add_mixin(&mut self, mixin: UnresolvedMixin) {
        self.mixins.push(mixin);
    }

    /// Add an interface declaration.
    pub fn add_interface(&mut self, interface: UnresolvedInterface) {
        self.interfaces.push(interface);
    }

    /// Add a funcdef declaration.
    pub fn add_funcdef(&mut self, funcdef: UnresolvedFuncdef) {
        self.funcdefs.push(funcdef);
    }

    /// Add an enum declaration.
    pub fn add_enum(&mut self, e: UnresolvedEnum) {
        self.enums.push(e);
    }

    /// Add a function declaration.
    pub fn add_function(&mut self, function: UnresolvedFunction) {
        self.functions.push(function);
    }

    /// Add a global variable declaration.
    pub fn add_global(&mut self, global: UnresolvedGlobal) {
        self.globals.push(global);
    }

    /// Add an error.
    pub fn add_error(&mut self, error: CompilationError) {
        self.errors.push(error);
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get total number of type declarations.
    pub fn type_count(&self) -> usize {
        self.classes.len()
            + self.mixins.len()
            + self.interfaces.len()
            + self.funcdefs.len()
            + self.enums.len()
    }

    /// Get total number of function declarations.
    pub fn function_count(&self) -> usize {
        self.functions.len()
            + self.classes.iter().map(|c| c.methods.len()).sum::<usize>()
    }

    /// Get total number of global variable declarations.
    pub fn global_count(&self) -> usize {
        self.globals.len()
    }

    /// Get all type names for duplicate detection during completion.
    pub fn all_type_names(&self) -> impl Iterator<Item = &QualifiedName> {
        self.classes
            .iter()
            .map(|c| &c.name)
            .chain(self.mixins.iter().map(|m| &m.class.name))
            .chain(self.interfaces.iter().map(|i| &i.name))
            .chain(self.funcdefs.iter().map(|f| &f.name))
            .chain(self.enums.iter().map(|e| &e.name))
    }

    /// Merge another registration result into this one.
    ///
    /// Used when combining results from multiple files in the same unit.
    pub fn merge(&mut self, other: RegistrationResult) {
        self.classes.extend(other.classes);
        self.mixins.extend(other.mixins);
        self.interfaces.extend(other.interfaces);
        self.funcdefs.extend(other.funcdefs);
        self.enums.extend(other.enums);
        self.functions.extend(other.functions);
        self.globals.extend(other.globals);
        self.errors.extend(other.errors);
    }
}
```

---

## Statistics Helper

```rust
/// Statistics from the registration pass.
#[derive(Debug, Default, Clone, Copy)]
pub struct RegistrationStats {
    pub classes: usize,
    pub mixins: usize,
    pub interfaces: usize,
    pub funcdefs: usize,
    pub enums: usize,
    pub functions: usize,
    pub globals: usize,
    pub errors: usize,
}

impl From<&RegistrationResult> for RegistrationStats {
    fn from(result: &RegistrationResult) -> Self {
        Self {
            classes: result.classes.len(),
            mixins: result.mixins.len(),
            interfaces: result.interfaces.len(),
            funcdefs: result.funcdefs.len(),
            enums: result.enums.len(),
            functions: result.functions.len(),
            globals: result.globals.len(),
            errors: result.errors.len(),
        }
    }
}

impl std::fmt::Display for RegistrationStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Registration: {} classes, {} mixins, {} interfaces, {} funcdefs, \
             {} enums, {} functions, {} globals, {} errors",
            self.classes,
            self.mixins,
            self.interfaces,
            self.funcdefs,
            self.enums,
            self.functions,
            self.globals,
            self.errors
        )
    }
}
```

---

## Module Updates

```rust
// crates/angelscript-compiler/src/passes/mod.rs

mod registration;
mod registration_result;
mod completion;
mod compilation;

pub use registration::RegistrationPass;
pub use registration_result::{RegistrationResult, RegistrationStats};
pub use completion::{CompletionPass, CompletionResult};
pub use compilation::CompilationPass;

// Remove PendingResolutions - no longer needed
// pub use registration::PendingResolutions;
```

---

## Usage Example

```rust
// How the passes connect:

fn compile_unit(ast: &Script, global_registry: &SymbolRegistry) -> Result<BytecodeModule, Vec<CompilationError>> {
    // Pass 1: Registration (pure function, no registry access)
    let registration_result = RegistrationPass::new(unit_id).run(ast);

    if registration_result.has_errors() {
        return Err(registration_result.errors);
    }

    // Pass 2: Completion (transforms + populates registry)
    let mut unit_registry = SymbolRegistry::new();
    let completion_result = CompletionPass::new(&mut unit_registry, global_registry)
        .run(registration_result)?;

    // Pass 3: Compilation (uses fully resolved registry)
    let bytecode = CompilationPass::new(&unit_registry, global_registry)
        .run(ast)?;

    Ok(bytecode)
}
```

---

## Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{QualifiedName, Span, UnitId};

    #[test]
    fn empty_result() {
        let result = RegistrationResult::new(UnitId::new(0));
        assert_eq!(result.type_count(), 0);
        assert_eq!(result.function_count(), 0);
        assert!(!result.has_errors());
    }

    #[test]
    fn add_class() {
        let mut result = RegistrationResult::new(UnitId::new(0));
        let class = UnresolvedClass::new(
            QualifiedName::global("Player"),
            Span::default(),
            UnitId::new(0),
        );
        result.add_class(class);

        assert_eq!(result.type_count(), 1);
        assert_eq!(result.classes.len(), 1);
    }

    #[test]
    fn merge_results() {
        let mut result1 = RegistrationResult::new(UnitId::new(0));
        result1.add_class(UnresolvedClass::new(
            QualifiedName::global("Foo"),
            Span::default(),
            UnitId::new(0),
        ));

        let mut result2 = RegistrationResult::new(UnitId::new(0));
        result2.add_class(UnresolvedClass::new(
            QualifiedName::global("Bar"),
            Span::default(),
            UnitId::new(0),
        ));

        result1.merge(result2);
        assert_eq!(result1.classes.len(), 2);
    }

    #[test]
    fn all_type_names() {
        let mut result = RegistrationResult::new(UnitId::new(0));

        result.add_class(UnresolvedClass::new(
            QualifiedName::global("MyClass"),
            Span::default(),
            UnitId::new(0),
        ));

        result.add_interface(UnresolvedInterface::new(
            QualifiedName::global("IMyInterface"),
            Span::default(),
            UnitId::new(0),
        ));

        let names: Vec<_> = result.all_type_names().collect();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn stats() {
        let mut result = RegistrationResult::new(UnitId::new(0));
        result.add_class(UnresolvedClass::new(
            QualifiedName::global("A"),
            Span::default(),
            UnitId::new(0),
        ));
        result.add_function(UnresolvedFunction::new(
            QualifiedName::global("foo"),
            Span::default(),
            UnitId::new(0),
            UnresolvedSignature::new("foo", vec![], UnresolvedType::simple("void")),
        ));

        let stats = RegistrationStats::from(&result);
        assert_eq!(stats.classes, 1);
        assert_eq!(stats.functions, 1);
    }
}
```

---

## Key Design Decisions

### Why a separate struct instead of tuples?

1. **Named fields** - Clear what each part contains
2. **Helper methods** - `type_count()`, `has_errors()`, `merge()`
3. **Extensible** - Easy to add fields (e.g., imports, typedefs) later
4. **Documentation** - Can document the struct and its purpose

### Why collect errors here?

Registration can still produce errors (duplicate names, invalid syntax that the parser missed). These are collected in the result rather than stopping immediately, allowing us to report multiple errors at once.

### Why include unit_id?

The result needs to know which unit it came from for:
- Error reporting with source locations
- Determining where resolved entries should be registered
- Supporting multi-unit compilation

---

## What's Next

Phase 4 will update the registry to support `QualifiedName`-based lookup alongside `TypeHash`.
