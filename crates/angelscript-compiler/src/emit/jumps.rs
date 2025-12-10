//! Jump management for control flow.
//!
//! This module handles tracking breakable contexts (loops and switches) for
//! break/continue statements, managing forward jumps that need patching,
//! and backward jumps for loops.

use super::JumpLabel;

/// Manages jump targets for control flow.
///
/// Tracks a stack of breakable contexts to support nested loops and switches
/// with proper break/continue handling.
#[derive(Debug, Default)]
pub struct JumpManager {
    /// Stack of breakable contexts (innermost last)
    contexts: Vec<BreakableContext>,
}

/// The kind of breakable context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakableKind {
    /// A loop (while, for, do-while) - supports both break and continue
    Loop,
    /// A switch statement - supports only break
    Switch,
}

/// Context for a breakable construct (loop or switch).
#[derive(Debug)]
struct BreakableContext {
    /// What kind of breakable this is
    kind: BreakableKind,
    /// Target offset for continue statements (only valid for loops)
    continue_target: Option<usize>,
    /// Pending break jumps to patch when the context exits
    break_labels: Vec<JumpLabel>,
}

impl JumpManager {
    /// Create a new jump manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enter a new loop context.
    ///
    /// # Arguments
    /// * `continue_target` - The bytecode offset to jump to for continue statements
    pub fn enter_loop(&mut self, continue_target: usize) {
        self.contexts.push(BreakableContext {
            kind: BreakableKind::Loop,
            continue_target: Some(continue_target),
            break_labels: Vec::new(),
        });
    }

    /// Enter a new switch context.
    ///
    /// Switch statements support break but not continue.
    pub fn enter_switch(&mut self) {
        self.contexts.push(BreakableContext {
            kind: BreakableKind::Switch,
            continue_target: None,
            break_labels: Vec::new(),
        });
    }

    /// Exit the current breakable context (loop or switch).
    ///
    /// Returns the break labels that need to be patched to jump past the construct.
    pub fn exit_breakable(&mut self) -> Vec<JumpLabel> {
        self.contexts
            .pop()
            .map(|ctx| ctx.break_labels)
            .unwrap_or_default()
    }

    /// Exit the current loop context.
    ///
    /// This is an alias for `exit_breakable()` for backwards compatibility.
    pub fn exit_loop(&mut self) -> Vec<JumpLabel> {
        self.exit_breakable()
    }

    /// Exit the current switch context.
    ///
    /// This is an alias for `exit_breakable()` for clarity.
    pub fn exit_switch(&mut self) -> Vec<JumpLabel> {
        self.exit_breakable()
    }

    /// Check if we're currently inside a loop.
    ///
    /// This checks if any context in the stack is a loop.
    pub fn in_loop(&self) -> bool {
        self.contexts
            .iter()
            .any(|ctx| ctx.kind == BreakableKind::Loop)
    }

    /// Check if we're currently inside a switch.
    pub fn in_switch(&self) -> bool {
        self.contexts
            .iter()
            .any(|ctx| ctx.kind == BreakableKind::Switch)
    }

    /// Check if we're inside any breakable context (loop or switch).
    pub fn in_breakable(&self) -> bool {
        !self.contexts.is_empty()
    }

    /// Add a break label to be patched when the innermost breakable exits.
    pub fn add_break(&mut self, label: JumpLabel) {
        if let Some(ctx) = self.contexts.last_mut() {
            ctx.break_labels.push(label);
        }
    }

    /// Get the continue target for the innermost loop.
    ///
    /// Returns an error if not inside a loop (switches don't have continue).
    pub fn continue_target(&self) -> Result<usize, super::BreakError> {
        // Find the innermost loop (skip any switches)
        for ctx in self.contexts.iter().rev() {
            if ctx.kind == BreakableKind::Loop {
                return ctx.continue_target.ok_or(super::BreakError::NotInLoop);
            }
        }
        Err(super::BreakError::NotInLoop)
    }

    /// Update the continue target for the innermost loop.
    ///
    /// This is useful for `for` loops where the continue target is
    /// the update expression, not the condition.
    pub fn set_continue_target(&mut self, target: usize) {
        // Find the innermost loop
        for ctx in self.contexts.iter_mut().rev() {
            if ctx.kind == BreakableKind::Loop {
                ctx.continue_target = Some(target);
                return;
            }
        }
    }

    /// Get the current loop nesting depth (loops only, not switches).
    pub fn loop_depth(&self) -> usize {
        self.contexts
            .iter()
            .filter(|ctx| ctx.kind == BreakableKind::Loop)
            .count()
    }

    /// Get the current breakable nesting depth (loops and switches).
    pub fn breakable_depth(&self) -> usize {
        self.contexts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_manager_not_in_loop() {
        let manager = JumpManager::new();
        assert!(!manager.in_loop());
        assert!(!manager.in_switch());
        assert!(!manager.in_breakable());
        assert_eq!(manager.loop_depth(), 0);
        assert_eq!(manager.breakable_depth(), 0);
    }

    #[test]
    fn enter_loop() {
        let mut manager = JumpManager::new();
        manager.enter_loop(10);

        assert!(manager.in_loop());
        assert!(!manager.in_switch());
        assert!(manager.in_breakable());
        assert_eq!(manager.loop_depth(), 1);
        assert_eq!(manager.breakable_depth(), 1);
        assert_eq!(manager.continue_target(), Ok(10));
    }

    #[test]
    fn enter_switch() {
        let mut manager = JumpManager::new();
        manager.enter_switch();

        assert!(!manager.in_loop());
        assert!(manager.in_switch());
        assert!(manager.in_breakable());
        assert_eq!(manager.loop_depth(), 0);
        assert_eq!(manager.breakable_depth(), 1);
        // Continue should error - switches don't have continue
        assert!(manager.continue_target().is_err());
    }

    #[test]
    fn nested_loops() {
        let mut manager = JumpManager::new();
        manager.enter_loop(10);
        manager.enter_loop(20);

        assert_eq!(manager.loop_depth(), 2);
        assert_eq!(manager.continue_target(), Ok(20)); // Inner loop

        manager.exit_loop();
        assert_eq!(manager.loop_depth(), 1);
        assert_eq!(manager.continue_target(), Ok(10)); // Outer loop
    }

    #[test]
    fn switch_inside_loop() {
        let mut manager = JumpManager::new();
        manager.enter_loop(10);
        manager.enter_switch();

        assert!(manager.in_loop());
        assert!(manager.in_switch());
        assert_eq!(manager.loop_depth(), 1);
        assert_eq!(manager.breakable_depth(), 2);
        // Continue should find the outer loop
        assert_eq!(manager.continue_target(), Ok(10));

        manager.exit_switch();
        assert!(manager.in_loop());
        assert!(!manager.in_switch());
    }

    #[test]
    fn loop_inside_switch() {
        let mut manager = JumpManager::new();
        manager.enter_switch();
        manager.enter_loop(20);

        assert!(manager.in_loop());
        assert!(manager.in_switch());
        assert_eq!(manager.loop_depth(), 1);
        assert_eq!(manager.breakable_depth(), 2);
        assert_eq!(manager.continue_target(), Ok(20));

        manager.exit_loop();
        assert!(!manager.in_loop());
        assert!(manager.in_switch());
    }

    #[test]
    fn break_in_switch() {
        let mut manager = JumpManager::new();
        manager.enter_switch();
        manager.add_break(JumpLabel(100));
        manager.add_break(JumpLabel(110));

        let breaks = manager.exit_switch();
        assert_eq!(breaks.len(), 2);
        assert_eq!(breaks[0].0, 100);
        assert_eq!(breaks[1].0, 110);
    }

    #[test]
    fn break_targets_innermost() {
        let mut manager = JumpManager::new();
        manager.enter_loop(10);
        manager.add_break(JumpLabel(100)); // Break for outer loop

        manager.enter_switch();
        manager.add_break(JumpLabel(200)); // Break for switch

        // Exit switch - should only get switch's break
        let switch_breaks = manager.exit_switch();
        assert_eq!(switch_breaks.len(), 1);
        assert_eq!(switch_breaks[0].0, 200);

        // Exit loop - should get loop's break
        let loop_breaks = manager.exit_loop();
        assert_eq!(loop_breaks.len(), 1);
        assert_eq!(loop_breaks[0].0, 100);
    }

    #[test]
    fn exit_loop_returns_breaks() {
        let mut manager = JumpManager::new();
        manager.enter_loop(10);
        manager.add_break(JumpLabel(100));
        manager.add_break(JumpLabel(110));

        let breaks = manager.exit_loop();
        assert_eq!(breaks.len(), 2);
        assert_eq!(breaks[0].0, 100);
        assert_eq!(breaks[1].0, 110);
    }

    #[test]
    fn continue_target_error_outside_loop() {
        let manager = JumpManager::new();
        assert!(manager.continue_target().is_err());
    }

    #[test]
    fn continue_target_error_in_switch_only() {
        let mut manager = JumpManager::new();
        manager.enter_switch();
        // Continue should error when only in switch (no enclosing loop)
        assert!(manager.continue_target().is_err());
    }

    #[test]
    fn exit_empty_returns_empty() {
        let mut manager = JumpManager::new();
        let breaks = manager.exit_loop();
        assert!(breaks.is_empty());
    }

    #[test]
    fn set_continue_target() {
        let mut manager = JumpManager::new();
        manager.enter_loop(10);

        // Update continue target (for 'for' loop update expression)
        manager.set_continue_target(50);
        assert_eq!(manager.continue_target(), Ok(50));
    }

    #[test]
    fn set_continue_target_skips_switch() {
        let mut manager = JumpManager::new();
        manager.enter_loop(10);
        manager.enter_switch();

        // Should update the loop's continue target, not the switch
        manager.set_continue_target(50);
        manager.exit_switch();
        assert_eq!(manager.continue_target(), Ok(50));
    }
}
