//! Jump management for control flow.
//!
//! This module handles tracking loop contexts for break/continue statements,
//! managing forward jumps that need patching, and backward jumps for loops.

use super::JumpLabel;

/// Manages jump targets for control flow.
///
/// Tracks a stack of loop contexts to support nested loops with
/// proper break/continue handling.
#[derive(Debug, Default)]
pub struct JumpManager {
    /// Stack of loop contexts (innermost last)
    loops: Vec<LoopContext>,
}

/// Context for a single loop.
#[derive(Debug)]
struct LoopContext {
    /// Target offset for continue statements (loop start)
    continue_target: usize,
    /// Pending break jumps to patch when loop exits
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
        self.loops.push(LoopContext {
            continue_target,
            break_labels: Vec::new(),
        });
    }

    /// Exit the current loop context.
    ///
    /// Returns the break labels that need to be patched to jump past the loop.
    pub fn exit_loop(&mut self) -> Vec<JumpLabel> {
        self.loops
            .pop()
            .map(|ctx| ctx.break_labels)
            .unwrap_or_default()
    }

    /// Check if we're currently inside a loop.
    pub fn in_loop(&self) -> bool {
        !self.loops.is_empty()
    }

    /// Add a break label to be patched when the loop exits.
    pub fn add_break(&mut self, label: JumpLabel) {
        if let Some(ctx) = self.loops.last_mut() {
            ctx.break_labels.push(label);
        }
    }

    /// Get the continue target for the current loop.
    ///
    /// Returns an error if not inside a loop.
    pub fn continue_target(&self) -> Result<usize, super::BreakError> {
        self.loops
            .last()
            .map(|ctx| ctx.continue_target)
            .ok_or(super::BreakError::NotInLoop)
    }

    /// Get the current loop nesting depth.
    pub fn loop_depth(&self) -> usize {
        self.loops.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_manager_not_in_loop() {
        let manager = JumpManager::new();
        assert!(!manager.in_loop());
        assert_eq!(manager.loop_depth(), 0);
    }

    #[test]
    fn enter_loop() {
        let mut manager = JumpManager::new();
        manager.enter_loop(10);

        assert!(manager.in_loop());
        assert_eq!(manager.loop_depth(), 1);
        assert_eq!(manager.continue_target(), Ok(10));
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
    fn exit_empty_returns_empty() {
        let mut manager = JumpManager::new();
        let breaks = manager.exit_loop();
        assert!(breaks.is_empty());
    }
}
