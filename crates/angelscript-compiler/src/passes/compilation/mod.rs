//! Pass 2: Compilation
//!
//! Type checks function bodies and generates bytecode.
//! Split into independently testable components:
//!
//! - `expr_checker`: Expression type checking
//! - `stmt_compiler`: Statement compilation
//! - `overload`: Function overload resolution
//! - `call_checker`: Function/method call checking
//! - `op_checker`: Operator overload checking
//! - `member_checker`: Member access checking
//! - `lambda`: Lambda compilation
