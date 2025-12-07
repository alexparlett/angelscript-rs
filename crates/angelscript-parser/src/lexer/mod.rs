//! Internal lexical analysis for AngelScript.

mod cursor;
mod error;
mod lexer;
mod token;

pub use angelscript_core::Span;
pub use lexer::Lexer;
pub use token::{Token, TokenKind};
