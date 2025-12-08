# Task 38: Local Scope & Variables

## Overview

Implement local scope management for tracking variables during function compilation. This includes variable declaration, scope push/pop, shadowing, and lambda captures.

## Goals

1. Track local variables with type and slot information
2. Support nested scopes with proper shadowing
3. Handle variable mutability (const)
4. Support lambda variable capture
5. Compute stack slot allocation

## Dependencies

- Task 31: Compiler Foundation

## Files to Create

```
crates/angelscript-compiler/src/
├── scope.rs               # LocalScope implementation
└── lib.rs
```

## Detailed Implementation

### LocalScope (scope.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash};
use rustc_hash::FxHashMap;

/// Information about a local variable.
#[derive(Debug, Clone)]
pub struct LocalVar {
    /// Variable name
    pub name: String,
    /// Variable type
    pub data_type: DataType,
    /// Stack slot index
    pub slot: u32,
    /// Scope depth where declared
    pub depth: u32,
    /// Whether variable is const
    pub is_const: bool,
    /// Whether variable has been initialized
    pub is_initialized: bool,
    /// Source location of declaration
    pub span: Span,
}

/// Information about a captured variable (for lambdas).
#[derive(Debug, Clone)]
pub struct CapturedVar {
    /// Original variable name
    pub name: String,
    /// Type of captured variable
    pub data_type: DataType,
    /// Index in capture array
    pub capture_index: usize,
    /// Whether captured by reference
    pub by_reference: bool,
    /// Original slot (if from enclosing function)
    pub original_slot: Option<u32>,
    /// Whether the original is const
    pub is_const: bool,
}

/// Local scope for a function being compiled.
#[derive(Debug)]
pub struct LocalScope {
    /// Variables by name in current scope chain
    variables: FxHashMap<String, LocalVar>,

    /// Current scope depth (0 = function scope)
    scope_depth: u32,

    /// Stack of shadowed variables (name, old_var, depth)
    /// When we shadow a variable, we save the old one here
    shadowed: Vec<(String, LocalVar)>,

    /// Next available stack slot
    next_slot: u32,

    /// Maximum slot used (for stack frame size)
    max_slot: u32,

    /// Captured variables (for lambdas)
    captures: Vec<CapturedVar>,

    /// Parent scope (for nested functions/lambdas)
    parent: Option<Box<LocalScope>>,
}

impl LocalScope {
    /// Create a new local scope for a function.
    pub fn new() -> Self {
        Self {
            variables: FxHashMap::default(),
            scope_depth: 0,
            shadowed: Vec::new(),
            next_slot: 0,
            max_slot: 0,
            captures: Vec::new(),
            parent: None,
        }
    }

    /// Create a nested scope (for lambdas).
    pub fn nested(parent: LocalScope) -> Self {
        Self {
            variables: FxHashMap::default(),
            scope_depth: 0,
            shadowed: Vec::new(),
            next_slot: 0,
            max_slot: 0,
            captures: Vec::new(),
            parent: Some(Box::new(parent)),
        }
    }

    // ==========================================================================
    // Scope Management
    // ==========================================================================

    /// Enter a new scope (block, if body, loop body, etc.).
    pub fn push_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// Exit the current scope, removing variables declared in it.
    pub fn pop_scope(&mut self) {
        // Remove all variables declared at current depth
        self.variables.retain(|_, var| var.depth < self.scope_depth);

        // Restore any shadowed variables
        while let Some((name, var)) = self.shadowed.last() {
            if var.depth >= self.scope_depth {
                let (name, var) = self.shadowed.pop().unwrap();
                self.variables.insert(name, var);
            } else {
                break;
            }
        }

        // Note: we don't decrease next_slot here because stack slots
        // can be reused but the max must account for all simultaneously live vars

        self.scope_depth -= 1;
    }

    /// Get current scope depth.
    pub fn depth(&self) -> u32 {
        self.scope_depth
    }

    // ==========================================================================
    // Variable Declaration
    // ==========================================================================

    /// Declare a new local variable.
    ///
    /// Returns the slot number, or error if already declared at same depth.
    pub fn declare(
        &mut self,
        name: String,
        data_type: DataType,
        is_const: bool,
        span: Span,
    ) -> Result<u32, ScopeError> {
        // Check for redeclaration at same scope depth
        if let Some(existing) = self.variables.get(&name) {
            if existing.depth == self.scope_depth {
                return Err(ScopeError::Redeclaration {
                    name: name.clone(),
                    original_span: existing.span,
                    new_span: span,
                });
            }
            // Shadowing is allowed - save the old variable
            self.shadowed.push((name.clone(), existing.clone()));
        }

        let slot = self.allocate_slot();

        let var = LocalVar {
            name: name.clone(),
            data_type,
            slot,
            depth: self.scope_depth,
            is_const,
            is_initialized: false,
            span,
        };

        self.variables.insert(name, var);

        Ok(slot)
    }

    /// Declare a function parameter.
    ///
    /// Parameters are at depth 0 and always initialized.
    pub fn declare_param(
        &mut self,
        name: String,
        data_type: DataType,
        is_const: bool,
        span: Span,
    ) -> Result<u32, ScopeError> {
        if self.variables.contains_key(&name) {
            return Err(ScopeError::Redeclaration {
                name: name.clone(),
                original_span: span,
                new_span: span,
            });
        }

        let slot = self.allocate_slot();

        let var = LocalVar {
            name: name.clone(),
            data_type,
            slot,
            depth: 0,
            is_const,
            is_initialized: true,  // Parameters are always initialized
            span,
        };

        self.variables.insert(name, var);

        Ok(slot)
    }

    /// Mark a variable as initialized.
    pub fn mark_initialized(&mut self, name: &str) {
        if let Some(var) = self.variables.get_mut(name) {
            var.is_initialized = true;
        }
    }

    fn allocate_slot(&mut self) -> u32 {
        let slot = self.next_slot;
        self.next_slot += 1;
        self.max_slot = self.max_slot.max(self.next_slot);
        slot
    }

    // ==========================================================================
    // Variable Lookup
    // ==========================================================================

    /// Look up a variable by name.
    pub fn get(&self, name: &str) -> Option<&LocalVar> {
        self.variables.get(name)
    }

    /// Look up a variable, checking parent scopes for captures.
    pub fn get_or_capture(&mut self, name: &str) -> Option<VarLookup> {
        // First check local scope
        if let Some(var) = self.variables.get(name) {
            return Some(VarLookup::Local(var.clone()));
        }

        // Check if already captured
        if let Some(capture) = self.captures.iter().find(|c| c.name == name) {
            return Some(VarLookup::Captured(capture.clone()));
        }

        // Check parent scope and capture if found
        if let Some(parent) = &mut self.parent {
            if let Some(lookup) = parent.get_or_capture(name) {
                match lookup {
                    VarLookup::Local(var) => {
                        // Capture from parent
                        let capture = CapturedVar {
                            name: name.to_string(),
                            data_type: var.data_type,
                            capture_index: self.captures.len(),
                            by_reference: true,  // Default to by-reference capture
                            original_slot: Some(var.slot),
                            is_const: var.is_const,
                        };
                        self.captures.push(capture.clone());
                        return Some(VarLookup::Captured(capture));
                    }
                    VarLookup::Captured(cap) => {
                        // Re-capture from parent's captures
                        let capture = CapturedVar {
                            name: name.to_string(),
                            data_type: cap.data_type,
                            capture_index: self.captures.len(),
                            by_reference: cap.by_reference,
                            original_slot: None,
                            is_const: cap.is_const,
                        };
                        self.captures.push(capture.clone());
                        return Some(VarLookup::Captured(capture));
                    }
                }
            }
        }

        None
    }

    /// Check if a name is declared in the current scope (not parent scopes).
    pub fn is_declared_in_current_scope(&self, name: &str) -> bool {
        self.variables.get(name)
            .map(|v| v.depth == self.scope_depth)
            .unwrap_or(false)
    }

    // ==========================================================================
    // Accessors
    // ==========================================================================

    /// Get the maximum stack frame size needed.
    pub fn frame_size(&self) -> u32 {
        self.max_slot
    }

    /// Get all captured variables.
    pub fn captures(&self) -> &[CapturedVar] {
        &self.captures
    }

    /// Check if there are any captures (i.e., this is a closure).
    pub fn has_captures(&self) -> bool {
        !self.captures.is_empty()
    }

    /// Iterate over all variables in scope.
    pub fn iter(&self) -> impl Iterator<Item = &LocalVar> {
        self.variables.values()
    }

    /// Take the parent scope (for returning from lambda compilation).
    pub fn take_parent(&mut self) -> Option<LocalScope> {
        self.parent.take().map(|b| *b)
    }
}

/// Result of variable lookup.
#[derive(Debug, Clone)]
pub enum VarLookup {
    /// Variable found in local scope
    Local(LocalVar),
    /// Variable captured from enclosing scope
    Captured(CapturedVar),
}

/// Scope-related errors.
#[derive(Debug, Clone)]
pub enum ScopeError {
    /// Variable redeclared in same scope
    Redeclaration {
        name: String,
        original_span: Span,
        new_span: Span,
    },
}

impl Default for LocalScope {
    fn default() -> Self {
        Self::new()
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declare_variable() {
        let mut scope = LocalScope::new();
        let slot = scope.declare("x".into(), DataType::simple(primitives::INT32), false, Span::default());
        assert!(slot.is_ok());
        assert_eq!(slot.unwrap(), 0);

        let var = scope.get("x");
        assert!(var.is_some());
        assert_eq!(var.unwrap().slot, 0);
    }

    #[test]
    fn redeclaration_error() {
        let mut scope = LocalScope::new();
        scope.declare("x".into(), DataType::simple(primitives::INT32), false, Span::default()).unwrap();
        let result = scope.declare("x".into(), DataType::simple(primitives::INT32), false, Span::default());
        assert!(matches!(result, Err(ScopeError::Redeclaration { .. })));
    }

    #[test]
    fn shadowing_allowed() {
        let mut scope = LocalScope::new();
        scope.declare("x".into(), DataType::simple(primitives::INT32), false, Span::default()).unwrap();

        scope.push_scope();
        let result = scope.declare("x".into(), DataType::simple(primitives::FLOAT), false, Span::default());
        assert!(result.is_ok());

        // Inner x is float
        let var = scope.get("x").unwrap();
        assert_eq!(var.data_type.type_hash, primitives::FLOAT);

        scope.pop_scope();

        // Outer x is int again
        let var = scope.get("x").unwrap();
        assert_eq!(var.data_type.type_hash, primitives::INT32);
    }

    #[test]
    fn scope_pop_removes_vars() {
        let mut scope = LocalScope::new();
        scope.push_scope();
        scope.declare("x".into(), DataType::simple(primitives::INT32), false, Span::default()).unwrap();
        scope.pop_scope();

        assert!(scope.get("x").is_none());
    }

    #[test]
    fn lambda_capture() {
        let mut outer = LocalScope::new();
        outer.declare("x".into(), DataType::simple(primitives::INT32), false, Span::default()).unwrap();

        let mut inner = LocalScope::nested(outer);

        let lookup = inner.get_or_capture("x");
        assert!(matches!(lookup, Some(VarLookup::Captured(_))));
        assert_eq!(inner.captures().len(), 1);
    }
}
```

## Acceptance Criteria

- [ ] Variables can be declared with unique slots
- [ ] Redeclaration in same scope is an error
- [ ] Shadowing in nested scope works correctly
- [ ] Scope pop removes variables and restores shadowed
- [ ] Parameters are handled correctly
- [ ] Lambda captures work across scope boundaries
- [ ] Frame size correctly tracks maximum slots
- [ ] All tests pass

## Next Phase

Task 39: Bytecode Emitter - instruction emission and jump management
