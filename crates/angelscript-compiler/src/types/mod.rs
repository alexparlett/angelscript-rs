//! Core type definitions for the compiler.

pub mod type_hash;
mod data_type;
mod type_def;
mod function_def;
mod expr_info;

pub use type_hash::{hash_constants, primitives, TypeHash};
pub use data_type::DataType;
pub use type_def::TypeDef;
pub use function_def::FunctionDef;
pub use expr_info::ExprInfo;
