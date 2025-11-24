//! Internal lexical analysis for AngelScript.

mod cursor;
mod error;
mod lexer;
mod span;
mod token;

pub use lexer::Lexer;
pub use span::Span;
pub use token::{Token, TokenKind};
