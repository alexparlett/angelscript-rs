//! Internal lexical analysis for AngelScript.

mod cursor;
mod lexer;
mod token;

pub use angelscript_core::{LexError, Span};
pub use lexer::Lexer;
pub use token::{Token, TokenKind};
