//! Local variable scope tracking for function compilation.
//!
//! This module provides per-function local variable tracking with support for:
//! - Nested scopes (blocks)
//! - Variable shadowing
//! - Stack offset management
//! - Scope depth tracking

use rustc_hash::FxHashMap;

use crate::semantic::DataType;

/// Tracks local variables for a single function compilation.
///
/// This structure maintains the local variable state during compilation of a single function.
/// It is created fresh for each function and discarded after compilation completes.
///
/// # Design
///
/// - Variables are stored in a hash map keyed by name
/// - When a scope exits, variables at that scope depth are removed
/// - Variable shadowing is allowed - inner scope variables hide outer scope variables
/// - Stack offsets are managed to track where variables live on the stack
///
/// # Example
///
/// ```
/// # use angelscript::semantic::{LocalScope, DataType, TypeId};
/// let mut scope = LocalScope::new();
///
/// // Enter a new scope
/// scope.enter_scope();
///
/// // Declare a variable
/// let int_type = DataType::simple(TypeId(4)); // int type
/// scope.declare_variable("x".to_string(), int_type.clone(), 0, true);
///
/// // Look up the variable
/// assert!(scope.lookup("x").is_some());
///
/// // Exit the scope - variable is removed
/// scope.exit_scope();
/// assert!(scope.lookup("x").is_none());
/// ```
#[derive(Debug, Clone)]
pub struct LocalScope {
    /// Variables in current function (name → info)
    /// The hash map stores the most recent declaration of each variable name.
    /// When shadowing occurs, the outer variable is temporarily replaced.
    variables: FxHashMap<String, LocalVar>,

    /// Current scope depth (0 = function parameters, 1+ = nested blocks)
    scope_depth: u32,

    /// Stack of shadowed variables (to restore when exiting scopes)
    /// When a variable shadows another, we push the outer one here.
    shadowed: Vec<(String, LocalVar, u32)>, // (name, var, scope_depth_when_shadowed)

    /// Next available stack offset
    next_offset: u32,
}

/// Information about a local variable.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalVar {
    /// Variable name
    pub name: String,

    /// Variable type
    pub data_type: DataType,

    /// Scope depth where this variable was declared
    pub scope_depth: u32,

    /// Stack offset for this variable
    pub stack_offset: u32,

    /// Whether the variable is mutable
    pub is_mutable: bool,
}

impl LocalScope {
    /// Creates a new empty local scope at depth 0.
    ///
    /// This should be called once per function compilation.
    pub fn new() -> Self {
        Self {
            variables: FxHashMap::default(),
            scope_depth: 0,
            shadowed: Vec::new(),
            next_offset: 0,
        }
    }

    /// Enters a new nested scope (increments depth).
    ///
    /// Should be called when entering a new block (e.g., function body, if statement, loop).
    ///
    /// # Example
    ///
    /// ```
    /// # use angelscript::semantic::LocalScope;
    /// let mut scope = LocalScope::new();
    /// assert_eq!(scope.scope_depth(), 0);
    ///
    /// scope.enter_scope();
    /// assert_eq!(scope.scope_depth(), 1);
    /// ```
    pub fn enter_scope(&mut self) {
        self.scope_depth += 1;
    }

    /// Exits the current scope (decrements depth).
    ///
    /// Removes all variables declared in the current scope and restores shadowed variables.
    ///
    /// # Panics
    ///
    /// Panics if called when scope_depth is 0.
    ///
    /// # Example
    ///
    /// ```
    /// # use angelscript::semantic::{LocalScope, DataType, TypeId};
    /// let mut scope = LocalScope::new();
    /// scope.enter_scope();
    ///
    /// // Declare a variable in the inner scope
    /// let int_type = DataType::simple(TypeId(4));
    /// scope.declare_variable("x".to_string(), int_type, 0, true);
    /// assert!(scope.lookup("x").is_some());
    ///
    /// // Exit scope - variable is removed
    /// scope.exit_scope();
    /// assert!(scope.lookup("x").is_none());
    /// ```
    pub fn exit_scope(&mut self) {
        assert!(self.scope_depth > 0, "Cannot exit scope at depth 0");

        // Remove all variables declared at this scope depth
        self.variables
            .retain(|_, v| v.scope_depth < self.scope_depth);

        // Restore shadowed variables from this scope
        let mut i = 0;
        while i < self.shadowed.len() {
            let (_, _, shadowed_at_depth) = &self.shadowed[i];
            if *shadowed_at_depth == self.scope_depth {
                // Restore this variable
                let (name, var, _) = self.shadowed.remove(i);
                self.variables.insert(name, var);
                // Don't increment i, we just removed an element
            } else {
                i += 1;
            }
        }

        self.scope_depth -= 1;
    }

    /// Declares a new variable in the current scope.
    ///
    /// If a variable with the same name exists in an outer scope, it will be shadowed.
    /// If a variable with the same name exists in the current scope, it will be replaced.
    ///
    /// # Parameters
    ///
    /// - `name`: Variable name
    /// - `data_type`: Variable type
    /// - `stack_offset`: Stack offset for this variable
    /// - `is_mutable`: Whether the variable can be modified
    ///
    /// # Example
    ///
    /// ```
    /// # use angelscript::semantic::{LocalScope, DataType, TypeId};
    /// let mut scope = LocalScope::new();
    /// scope.enter_scope();
    ///
    /// let int_type = DataType::simple(TypeId(4));
    /// scope.declare_variable("x".to_string(), int_type.clone(), 0, true);
    ///
    /// let var = scope.lookup("x").unwrap();
    /// assert_eq!(var.name, "x");
    /// assert_eq!(var.stack_offset, 0);
    /// assert!(var.is_mutable);
    /// ```
    pub fn declare_variable(
        &mut self,
        name: String,
        data_type: DataType,
        stack_offset: u32,
        is_mutable: bool,
    ) {
        let new_var = LocalVar {
            name: name.clone(),
            data_type,
            scope_depth: self.scope_depth,
            stack_offset,
            is_mutable,
        };

        // If there's an existing variable with this name, shadow it
        if let Some(old_var) = self.variables.get(&name) {
            // Only shadow if it's from an outer scope
            if old_var.scope_depth < self.scope_depth {
                let shadowed_var = old_var.clone();
                self.shadowed
                    .push((name.clone(), shadowed_var, self.scope_depth));
            }
        }

        self.variables.insert(name, new_var);

        // Update next offset
        if stack_offset >= self.next_offset {
            self.next_offset = stack_offset + 1;
        }
    }

    /// Allocates a new stack offset and declares a variable.
    ///
    /// This is a convenience method that automatically assigns the next available stack offset.
    ///
    /// # Returns
    ///
    /// The stack offset that was assigned to the variable.
    ///
    /// # Example
    ///
    /// ```
    /// # use angelscript::semantic::{LocalScope, DataType, TypeId};
    /// let mut scope = LocalScope::new();
    /// scope.enter_scope();
    ///
    /// let int_type = DataType::simple(TypeId(4));
    /// let offset = scope.declare_variable_auto("x".to_string(), int_type, true);
    /// assert_eq!(offset, 0);
    ///
    /// let offset2 = scope.declare_variable_auto("y".to_string(), DataType::simple(TypeId(4)), true);
    /// assert_eq!(offset2, 1);
    /// ```
    pub fn declare_variable_auto(
        &mut self,
        name: String,
        data_type: DataType,
        is_mutable: bool,
    ) -> u32 {
        let offset = self.next_offset;
        self.declare_variable(name, data_type, offset, is_mutable);
        offset
    }

    /// Looks up a variable by name.
    ///
    /// Returns the most recent declaration (handles shadowing correctly).
    ///
    /// # Example
    ///
    /// ```
    /// # use angelscript::semantic::{LocalScope, DataType, TypeId};
    /// let mut scope = LocalScope::new();
    /// scope.enter_scope();
    ///
    /// let int_type = DataType::simple(TypeId(4));
    /// scope.declare_variable("x".to_string(), int_type, 0, true);
    ///
    /// assert!(scope.lookup("x").is_some());
    /// assert!(scope.lookup("y").is_none());
    /// ```
    pub fn lookup(&self, name: &str) -> Option<&LocalVar> {
        self.variables.get(name)
    }

    /// Gets the current scope depth.
    ///
    /// Depth 0 is the function level (parameters).
    /// Depth 1+ are nested blocks.
    pub fn scope_depth(&self) -> u32 {
        self.scope_depth
    }

    /// Gets the next available stack offset.
    ///
    /// This is useful for allocating space for temporary values.
    pub fn next_stack_offset(&self) -> u32 {
        self.next_offset
    }

    /// Checks if a variable is declared in the current scope (not outer scopes).
    ///
    /// # Example
    ///
    /// ```
    /// # use angelscript::semantic::{LocalScope, DataType, TypeId};
    /// let mut scope = LocalScope::new();
    /// let int_type = DataType::simple(TypeId(4));
    ///
    /// scope.enter_scope();
    /// scope.declare_variable("x".to_string(), int_type.clone(), 0, true);
    ///
    /// scope.enter_scope();
    /// assert!(!scope.is_declared_in_current_scope("x")); // x is in outer scope
    ///
    /// scope.declare_variable("y".to_string(), int_type, 1, true);
    /// assert!(scope.is_declared_in_current_scope("y")); // y is in current scope
    /// ```
    pub fn is_declared_in_current_scope(&self, name: &str) -> bool {
        self.variables
            .get(name)
            .map(|v| v.scope_depth == self.scope_depth)
            .unwrap_or(false)
    }

    /// Returns the number of variables currently in scope.
    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }
}

impl Default for LocalScope {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{FLOAT_TYPE, INT32_TYPE, TypeId};

    fn int_type() -> DataType {
        DataType::simple(INT32_TYPE)
    }

    fn float_type() -> DataType {
        DataType::simple(FLOAT_TYPE)
    }

    #[test]
    fn new_scope_starts_at_depth_0() {
        let scope = LocalScope::new();
        assert_eq!(scope.scope_depth(), 0);
    }

    #[test]
    fn enter_scope_increments_depth() {
        let mut scope = LocalScope::new();
        scope.enter_scope();
        assert_eq!(scope.scope_depth(), 1);
        scope.enter_scope();
        assert_eq!(scope.scope_depth(), 2);
    }

    #[test]
    fn exit_scope_decrements_depth() {
        let mut scope = LocalScope::new();
        scope.enter_scope();
        scope.enter_scope();
        scope.exit_scope();
        assert_eq!(scope.scope_depth(), 1);
        scope.exit_scope();
        assert_eq!(scope.scope_depth(), 0);
    }

    #[test]
    fn declare_variable_makes_it_accessible() {
        let mut scope = LocalScope::new();
        scope.enter_scope();
        scope.declare_variable("x".to_string(), int_type(), 0, true);

        let var = scope.lookup("x").unwrap();
        assert_eq!(var.name, "x");
        assert_eq!(var.stack_offset, 0);
        assert!(var.is_mutable);
    }

    #[test]
    fn lookup_nonexistent_variable_returns_none() {
        let scope = LocalScope::new();
        assert!(scope.lookup("x").is_none());
    }

    #[test]
    fn exit_scope_removes_variables() {
        let mut scope = LocalScope::new();
        scope.enter_scope();
        scope.declare_variable("x".to_string(), int_type(), 0, true);
        assert!(scope.lookup("x").is_some());

        scope.exit_scope();
        assert!(scope.lookup("x").is_none());
    }

    #[test]
    fn variable_shadowing_works() {
        let mut scope = LocalScope::new();

        // Declare x in outer scope
        scope.enter_scope();
        scope.declare_variable("x".to_string(), int_type(), 0, true);
        assert_eq!(scope.lookup("x").unwrap().data_type, int_type());

        // Shadow x in inner scope with different type
        scope.enter_scope();
        scope.declare_variable("x".to_string(), float_type(), 1, true);
        assert_eq!(scope.lookup("x").unwrap().data_type, float_type());

        // Exit inner scope - outer x is restored
        scope.exit_scope();
        assert_eq!(scope.lookup("x").unwrap().data_type, int_type());
    }

    #[test]
    fn multiple_variables_in_same_scope() {
        let mut scope = LocalScope::new();
        scope.enter_scope();

        scope.declare_variable("x".to_string(), int_type(), 0, true);
        scope.declare_variable("y".to_string(), float_type(), 1, true);

        assert!(scope.lookup("x").is_some());
        assert!(scope.lookup("y").is_some());
        assert_eq!(scope.variable_count(), 2);
    }

    #[test]
    fn nested_scopes_preserve_outer_variables() {
        let mut scope = LocalScope::new();
        scope.enter_scope();
        scope.declare_variable("x".to_string(), int_type(), 0, true);

        scope.enter_scope();
        scope.declare_variable("y".to_string(), float_type(), 1, true);

        // Both variables are accessible
        assert!(scope.lookup("x").is_some());
        assert!(scope.lookup("y").is_some());

        // Exit inner scope
        scope.exit_scope();
        assert!(scope.lookup("x").is_some());
        assert!(scope.lookup("y").is_none());
    }

    #[test]
    fn declare_variable_auto_assigns_offsets() {
        let mut scope = LocalScope::new();
        scope.enter_scope();

        let offset1 = scope.declare_variable_auto("x".to_string(), int_type(), true);
        let offset2 = scope.declare_variable_auto("y".to_string(), float_type(), true);
        let offset3 = scope.declare_variable_auto("z".to_string(), int_type(), true);

        assert_eq!(offset1, 0);
        assert_eq!(offset2, 1);
        assert_eq!(offset3, 2);
    }

    #[test]
    fn next_stack_offset_tracks_correctly() {
        let mut scope = LocalScope::new();
        scope.enter_scope();

        assert_eq!(scope.next_stack_offset(), 0);
        scope.declare_variable("x".to_string(), int_type(), 0, true);
        assert_eq!(scope.next_stack_offset(), 1);
    }

    #[test]
    fn is_declared_in_current_scope_distinguishes_scopes() {
        let mut scope = LocalScope::new();
        scope.enter_scope();
        scope.declare_variable("x".to_string(), int_type(), 0, true);
        assert!(scope.is_declared_in_current_scope("x"));

        scope.enter_scope();
        assert!(!scope.is_declared_in_current_scope("x")); // x is in outer scope

        scope.declare_variable("y".to_string(), float_type(), 1, true);
        assert!(scope.is_declared_in_current_scope("y"));
    }

    #[test]
    fn shadowing_same_scope_replaces_variable() {
        let mut scope = LocalScope::new();
        scope.enter_scope();

        scope.declare_variable("x".to_string(), int_type(), 0, true);
        assert_eq!(scope.lookup("x").unwrap().data_type, int_type());

        // Redeclare in same scope - replaces
        scope.declare_variable("x".to_string(), float_type(), 0, false);
        assert_eq!(scope.lookup("x").unwrap().data_type, float_type());
        assert!(!scope.lookup("x").unwrap().is_mutable);
    }

    #[test]
    fn complex_shadowing_scenario() {
        let mut scope = LocalScope::new();

        // Depth 0
        scope.enter_scope();
        scope.declare_variable("x".to_string(), int_type(), 0, true);

        // Depth 1 - shadow x
        scope.enter_scope();
        scope.declare_variable("x".to_string(), float_type(), 1, true);
        assert_eq!(scope.lookup("x").unwrap().data_type, float_type());

        // Depth 2 - shadow x again
        scope.enter_scope();
        scope.declare_variable("x".to_string(), int_type(), 2, true);
        assert_eq!(scope.lookup("x").unwrap().data_type, int_type());
        assert_eq!(scope.lookup("x").unwrap().stack_offset, 2);

        // Exit to depth 1 - should restore float x
        scope.exit_scope();
        assert_eq!(scope.lookup("x").unwrap().data_type, float_type());
        assert_eq!(scope.lookup("x").unwrap().stack_offset, 1);

        // Exit to depth 0 - should restore int x
        scope.exit_scope();
        assert_eq!(scope.lookup("x").unwrap().data_type, int_type());
        assert_eq!(scope.lookup("x").unwrap().stack_offset, 0);
    }
}
