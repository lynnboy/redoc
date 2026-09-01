//! redoc-core: parser and AST for the redoc markup language.
//!
//! redoc is a markup language used by the loc-iso14882 project to carry
//! bilingual (English/Chinese) translations of the C++ standard.
//! Everything special lives inside `[...]`.

pub mod ast;
pub mod parser;
pub mod span;
pub mod token;
pub mod validate;

pub use ast::{Attr, Element, ElementKind, Node};
pub use parser::{parse, Document, ParseError};
pub use span::Span;
pub use token::{tokenize, Token};
pub use validate::{Diagnostic, Severity, Validation};
