//! Compilation context for managing multi-file script compilation.
//!
//! The CompilationContext owns the arena allocator and coordinates parsing
//! and semantic analysis across multiple script files.

use bumpalo::Bump;

use crate::ast::{parse, parse_lenient};

/// A compilation context that owns the arena allocator.
///
/// This allows multiple scripts to be parsed into the same arena,
/// enabling multi-file compilation where symbols from one file
/// can reference symbols from another file.
///
/// # Example
///
/// ```
/// use angelscript::CompilationContext;
///
/// let mut ctx = CompilationContext::new();
///
/// // Parse multiple files into the same context
/// let script1 = ctx.parse_script("class Player { int health; }").unwrap();
/// let script2 = ctx.parse_script("Player@ CreatePlayer() { return Player(); }").unwrap();
///
/// // Both scripts share the same arena and can reference each other's symbols
/// ```
pub struct CompilationContext {
    /// The arena allocator for all AST nodes.
    arena: Bump,
}

impl CompilationContext {
    /// Create a new compilation context with a fresh arena.
    pub fn new() -> Self {
        Self {
            arena: Bump::new(),
        }
    }

    /// Get a reference to the arena allocator.
    ///
    /// This allows parsing multiple scripts into the same arena.
    pub fn arena(&self) -> &Bump {
        &self.arena
    }

    /// Parse a script into this compilation context.
    ///
    /// The parsed AST will be allocated in this context's arena.
    /// Returns a Script that borrows from this context's arena.
    pub fn parse_script<'ctx>(
        &'ctx self,
        source: &'ctx str,
    ) -> Result<crate::ast::Script<'ctx, 'ctx>, crate::ast::ParseErrors> {
        parse(source, &self.arena)
    }

    /// Parse a script leniently into this compilation context.
    ///
    /// Always returns a Script even if errors occurred.
    pub fn parse_script_lenient<'ctx>(
        &'ctx self,
        source: &'ctx str,
    ) -> (crate::ast::Script<'ctx, 'ctx>, Vec<crate::ast::ParseError>) {
        parse_lenient(source, &self.arena)
    }

    /// Reset the arena, clearing all previously parsed scripts.
    ///
    /// This is useful when you want to reuse the context for a new
    /// compilation session without reallocating memory.
    pub fn reset(&mut self) {
        self.arena.reset();
    }
}

impl Default for CompilationContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_context() {
        let ctx = CompilationContext::new();
        assert!(ctx.arena().allocated_bytes() == 0);
    }

    #[test]
    fn parse_single_script() {
        let ctx = CompilationContext::new();
        let result = ctx.parse_script("void foo() {}");
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(script.items().len(), 1);
    }

    #[test]
    fn parse_multiple_scripts() {
        let ctx = CompilationContext::new();

        let script1 = ctx.parse_script("class Player { int health; }");
        assert!(script1.is_ok());

        let script2 = ctx.parse_script("void foo() {}");
        assert!(script2.is_ok());

        // Both scripts share the same arena
        let s1 = script1.unwrap();
        let s2 = script2.unwrap();
        assert_eq!(s1.items().len(), 1);
        assert_eq!(s2.items().len(), 1);
    }

    #[test]
    fn parse_lenient() {
        let ctx = CompilationContext::new();
        let (script, errors) = ctx.parse_script_lenient("void foo() {}");
        assert!(errors.is_empty());
        assert_eq!(script.items().len(), 1);
    }

    #[test]
    fn reset_arena() {
        let mut ctx = CompilationContext::new();

        // Parse a script
        let _ = ctx.parse_script("void foo() {}");
        let bytes_after_first_parse = ctx.arena().allocated_bytes();
        assert!(bytes_after_first_parse > 0);

        // Reset - note that bumpalo's reset() may keep memory allocated
        // but rewinds the allocation pointer for reuse
        ctx.reset();

        // Can parse again - this reuses the arena memory
        let result = ctx.parse_script("int x = 42;");
        assert!(result.is_ok());

        // Memory should be allocated again
        assert!(ctx.arena().allocated_bytes() > 0);
    }
}
