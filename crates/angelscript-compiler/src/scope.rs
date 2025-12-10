//! Local scope management for function compilation.
//!
//! This module provides `LocalScope` for tracking local variables during
//! function body compilation. It handles:
//! - Variable declaration with stack slot allocation
//! - Nested block scopes (if/while/for bodies)
//! - Variable shadowing with proper restoration on scope exit
//! - Lambda variable capture

use angelscript_core::{CompilationError, DataType, Span};
use rustc_hash::FxHashMap;

// ============================================================================
// Types
// ============================================================================

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

/// Result of variable lookup.
#[derive(Debug, Clone)]
pub enum VarLookup {
    /// Variable found in local scope
    Local(LocalVar),
    /// Variable captured from enclosing scope
    Captured(CapturedVar),
}

// ============================================================================
// LocalScope
// ============================================================================

/// Local scope for a function being compiled.
///
/// Tracks local variables, handles nested block scopes, and manages
/// lambda captures. This is separate from `Scope` which handles
/// namespace-level symbols (types, functions, globals).
#[derive(Debug)]
pub struct LocalScope {
    /// Variables by name in current scope chain
    variables: FxHashMap<String, LocalVar>,

    /// Current scope depth (0 = function scope)
    scope_depth: u32,

    /// Stack of shadowed variables (shadowing_depth, name, old_var)
    /// When we shadow a variable, we save the old one here along with the depth
    /// at which the shadowing occurred (not the depth of the original variable)
    shadowed: Vec<(u32, String, LocalVar)>,

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

        // Restore any shadowed variables from this depth
        // The shadowing_depth tells us when the shadowing occurred
        while let Some(&(shadowing_depth, _, _)) = self.shadowed.last() {
            if shadowing_depth == self.scope_depth {
                let (_, name, var) = self.shadowed.pop().unwrap();
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
    ) -> Result<u32, CompilationError> {
        // Check for redeclaration at same scope depth
        if let Some(existing) = self.variables.get(&name) {
            if existing.depth == self.scope_depth {
                return Err(CompilationError::VariableRedeclaration {
                    name: name.clone(),
                    original_span: existing.span,
                    new_span: span,
                });
            }
            // Shadowing is allowed - save the old variable with the current depth
            // (the depth at which shadowing is occurring)
            self.shadowed
                .push((self.scope_depth, name.clone(), existing.clone()));
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
    ) -> Result<u32, CompilationError> {
        if let Some(existing) = self.variables.get(&name) {
            return Err(CompilationError::VariableRedeclaration {
                name: name.clone(),
                original_span: existing.span,
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
            is_initialized: true, // Parameters are always initialized
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

    /// Look up a variable by name (local only, no capture).
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
        if let Some(parent) = &mut self.parent
            && let Some(lookup) = parent.get_or_capture(name)
        {
            match lookup {
                VarLookup::Local(var) => {
                    // Capture from parent
                    let capture = CapturedVar {
                        name: name.to_string(),
                        data_type: var.data_type,
                        capture_index: self.captures.len(),
                        by_reference: true, // Default to by-reference capture
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

        None
    }

    /// Check if a name is declared in the current scope (not parent scopes).
    pub fn is_declared_in_current_scope(&self, name: &str) -> bool {
        self.variables
            .get(name)
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

impl Default for LocalScope {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::primitives;

    #[test]
    fn declare_variable() {
        let mut scope = LocalScope::new();
        let slot = scope
            .declare(
                "x".into(),
                DataType::simple(primitives::INT32),
                false,
                Span::default(),
            )
            .unwrap();
        assert_eq!(slot, 0);

        let var = scope.get("x");
        assert!(var.is_some());
        assert_eq!(var.unwrap().slot, 0);
        assert!(!var.unwrap().is_initialized);
    }

    #[test]
    fn redeclaration_error() {
        let mut scope = LocalScope::new();
        scope
            .declare(
                "x".into(),
                DataType::simple(primitives::INT32),
                false,
                Span::default(),
            )
            .unwrap();

        let result = scope.declare(
            "x".into(),
            DataType::simple(primitives::INT32),
            false,
            Span::default(),
        );
        assert!(matches!(
            result,
            Err(CompilationError::VariableRedeclaration { .. })
        ));
    }

    #[test]
    fn shadowing_allowed() {
        let mut scope = LocalScope::new();
        scope
            .declare(
                "x".into(),
                DataType::simple(primitives::INT32),
                false,
                Span::default(),
            )
            .unwrap();

        scope.push_scope();
        let result = scope.declare(
            "x".into(),
            DataType::simple(primitives::FLOAT),
            false,
            Span::default(),
        );
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
        scope
            .declare(
                "x".into(),
                DataType::simple(primitives::INT32),
                false,
                Span::default(),
            )
            .unwrap();
        scope.pop_scope();

        assert!(scope.get("x").is_none());
    }

    #[test]
    fn frame_size_tracks_max() {
        let mut scope = LocalScope::new();
        scope
            .declare(
                "a".into(),
                DataType::simple(primitives::INT32),
                false,
                Span::default(),
            )
            .unwrap();
        scope
            .declare(
                "b".into(),
                DataType::simple(primitives::INT32),
                false,
                Span::default(),
            )
            .unwrap();

        scope.push_scope();
        scope
            .declare(
                "c".into(),
                DataType::simple(primitives::INT32),
                false,
                Span::default(),
            )
            .unwrap();
        scope.pop_scope();

        // Frame size should be 3 (max slots used)
        assert_eq!(scope.frame_size(), 3);
    }

    #[test]
    fn lambda_capture() {
        let mut outer = LocalScope::new();
        outer
            .declare(
                "x".into(),
                DataType::simple(primitives::INT32),
                false,
                Span::default(),
            )
            .unwrap();

        let mut inner = LocalScope::nested(outer);

        let lookup = inner.get_or_capture("x");
        assert!(matches!(lookup, Some(VarLookup::Captured(_))));
        assert_eq!(inner.captures().len(), 1);
        assert!(inner.has_captures());
    }

    #[test]
    fn params_are_initialized() {
        let mut scope = LocalScope::new();
        scope
            .declare_param(
                "arg".into(),
                DataType::simple(primitives::INT32),
                false,
                Span::default(),
            )
            .unwrap();

        let var = scope.get("arg").unwrap();
        assert!(var.is_initialized);
    }

    #[test]
    fn mark_initialized() {
        let mut scope = LocalScope::new();
        scope
            .declare(
                "x".into(),
                DataType::simple(primitives::INT32),
                false,
                Span::default(),
            )
            .unwrap();

        assert!(!scope.get("x").unwrap().is_initialized);
        scope.mark_initialized("x");
        assert!(scope.get("x").unwrap().is_initialized);
    }
}
