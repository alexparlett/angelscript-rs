//! Variable scope and stack allocation for the bytecode compiler.

use angelscript_core::DataType;

/// Tracks variable allocation within a function's stack frame.
#[derive(Debug)]
pub struct VariableScope {
    /// All variables in scope, in allocation order.
    variables: Vec<Variable>,
    /// Stack of scope boundaries (index into `variables`).
    scope_stack: Vec<usize>,
    /// Current stack offset in dwords (grows upward).
    next_offset: i16,
}

/// A local variable on the stack frame.
#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub data_type: DataType,
    /// Offset from frame pointer in dwords.
    pub offset: i16,
    /// Size in dwords on the stack.
    pub size: u16,
    /// Whether this variable is a temporary (compiler-generated).
    pub is_temp: bool,
    /// Scope depth at which this variable was allocated.
    pub scope_depth: usize,
}

impl VariableScope {
    pub fn new() -> Self {
        VariableScope {
            variables: Vec::new(),
            scope_stack: vec![0],
            next_offset: 0,
        }
    }

    /// Enter a new scope.
    pub fn push_scope(&mut self) {
        self.scope_stack.push(self.variables.len());
    }

    /// Leave the current scope, deallocating all variables in it.
    pub fn pop_scope(&mut self) {
        if let Some(boundary) = self.scope_stack.pop() {
            // Reclaim stack space.
            if let Some(last) = self.variables.get(boundary) {
                self.next_offset = last.offset;
            }
            self.variables.truncate(boundary);
        }
    }

    /// Allocate a named variable and return its stack offset.
    pub fn alloc_variable(&mut self, name: impl Into<String>, data_type: DataType) -> i16 {
        let size = data_type.stack_size_dwords().max(1) as u16;
        let offset = self.next_offset;
        self.next_offset += size as i16;

        self.variables.push(Variable {
            name: name.into(),
            data_type,
            offset,
            size,
            is_temp: false,
            scope_depth: self.scope_stack.len(),
        });

        offset
    }

    /// Allocate a temporary variable and return its stack offset.
    pub fn alloc_temp(&mut self, data_type: DataType) -> i16 {
        let size = data_type.stack_size_dwords().max(1) as u16;
        let offset = self.next_offset;
        self.next_offset += size as i16;

        self.variables.push(Variable {
            name: String::new(),
            data_type,
            offset,
            size,
            is_temp: true,
            scope_depth: self.scope_stack.len(),
        });

        offset
    }

    /// Look up a variable by name, searching from innermost scope outward.
    /// Skips temporaries (variables with empty names).
    pub fn lookup(&self, name: &str) -> Option<&Variable> {
        self.variables
            .iter()
            .rev()
            .find(|v| !v.name.is_empty() && v.name == name)
    }

    /// Get the total stack frame size in dwords.
    pub fn frame_size_dwords(&self) -> i16 {
        self.next_offset
    }

    /// Current scope depth.
    pub fn depth(&self) -> usize {
        self.scope_stack.len()
    }
}

impl Default for VariableScope {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::PrimitiveType;

    #[test]
    fn alloc_and_lookup() {
        let mut scope = VariableScope::new();
        let off = scope.alloc_variable("x", DataType::primitive(PrimitiveType::Int32));
        assert_eq!(off, 0);
        let v = scope.lookup("x").unwrap();
        assert_eq!(v.offset, 0);
        assert_eq!(v.size, 1);
    }

    #[test]
    fn multiple_variables() {
        let mut scope = VariableScope::new();
        scope.alloc_variable("a", DataType::primitive(PrimitiveType::Int32));
        scope.alloc_variable("b", DataType::primitive(PrimitiveType::Double));
        scope.alloc_variable("c", DataType::primitive(PrimitiveType::Bool));

        assert_eq!(scope.lookup("a").unwrap().offset, 0);
        assert_eq!(scope.lookup("b").unwrap().offset, 1); // int32 = 1 dword
        assert_eq!(scope.lookup("c").unwrap().offset, 3); // double = 2 dwords
    }

    #[test]
    fn scope_push_pop() {
        let mut scope = VariableScope::new();
        scope.alloc_variable("outer", DataType::primitive(PrimitiveType::Int32));

        scope.push_scope();
        scope.alloc_variable("inner", DataType::primitive(PrimitiveType::Int32));
        assert!(scope.lookup("inner").is_some());

        scope.pop_scope();
        assert!(scope.lookup("inner").is_none());
        assert!(scope.lookup("outer").is_some());
    }

    #[test]
    fn frame_size() {
        let mut scope = VariableScope::new();
        scope.alloc_variable("a", DataType::primitive(PrimitiveType::Int32));
        scope.alloc_variable("b", DataType::primitive(PrimitiveType::Int64));
        assert_eq!(scope.frame_size_dwords(), 3); // 1 + 2
    }

    #[test]
    fn temp_allocation() {
        let mut scope = VariableScope::new();
        let off = scope.alloc_temp(DataType::primitive(PrimitiveType::Int32));
        assert_eq!(off, 0);
        // Temps have empty names.
        assert!(scope.lookup("").is_none()); // lookup skips empty names
    }
}
