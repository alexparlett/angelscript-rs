pub mod gc;
pub mod memory;
pub mod vm;

// Re-export commonly used items
pub use gc::{GCFlags, GCStatistics, GarbageCollector};
pub use memory::{Object, ObjectHeap};
pub use vm::{StackFrame, VM, VMState};