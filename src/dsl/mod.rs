//! DSL (Domain Specific Language) parser for circuit descriptions.
//!
//! This module provides a SPICE-inspired text-based language for describing
//! guitar pedal circuits. The DSL is line-oriented and human-editable.
//!
//! # Grammar Overview
//!
//! ```text
//! circuit     = { line }
//! line        = comment | directive | component | empty
//! comment     = ('#' | ';') { any_char }
//! directive   = '.' directive_name { argument }
//! component   = type name node+ [value] [model_ref]
//!
//! directive_name = "node" | "model" | "input" | "output" | "param"
//! type        = "R" | "C" | "L" | "D" | "Q" | "V" | "I" | "OP" | "POT" | "SW"
//! name        = identifier
//! node        = identifier | "0" | "GND"
//! value       = number [unit_suffix]
//! model_ref   = identifier
//!
//! number      = ['-'] digit+ ['.' digit+] [('e'|'E') ['-'|'+'] digit+]
//! unit_suffix = 'p' | 'n' | 'u' | 'm' | 'k' | 'M' | 'G'
//! identifier  = (letter | '_') { letter | digit | '_' }
//! ```
//!
//! # Component Types
//!
//! | Type | Description | Syntax |
//! |------|-------------|--------|
//! | R | Resistor | `R<name> <n+> <n-> <value>` |
//! | C | Capacitor | `C<name> <n+> <n-> <value>` |
//! | L | Inductor | `L<name> <n+> <n-> <value>` |
//! | D | Diode | `D<name> <anode> <cathode> [model]` |
//! | Q | BJT | `Q<name> <collector> <base> <emitter> [model]` |
//! | V | Voltage Source | `V<name> <n+> <n-> <DC value> [AC amplitude]` |
//! | I | Current Source | `I<name> <n+> <n-> <value>` |
//! | OP | Op-Amp | `OP<name> <out> <in+> <in-> [model]` |
//! | POT | Potentiometer | `POT<name> <n1> <wiper> <n2> <value> <position>` |
//! | SW | Switch | `SW<name> <n1> <n2> <state>` |
//!
//! # Directives
//!
//! | Directive | Description | Syntax |
//! |-----------|-------------|--------|
//! | .node | Declare a node | `.node <name>` |
//! | .model | Define a component model | `.model <name> <type> (<params>)` |
//! | .input | Mark audio input node | `.input <node>` |
//! | .output | Mark audio output node | `.output <node>` |
//!
//! # Example
//!
//! ```text
//! # RC Low-pass filter
//! .input in
//! .output out
//!
//! VIN  in   0    AC 1.0
//! R1   in   out  10k
//! C1   out  0    100n
//! ```

mod ast;
mod lexer;
mod parser;

pub use ast::*;
pub use lexer::{Lexer, Token, TokenKind};
pub use parser::Parser;

use crate::error::Result;

/// Parse a circuit DSL string into an AST.
pub fn parse(input: &str) -> Result<CircuitAst> {
    let lexer = Lexer::new(input);
    let mut parser = Parser::new(lexer);
    parser.parse()
}

/// Parse a circuit DSL file.
#[cfg(feature = "cli")]
pub fn parse_file(path: &std::path::Path) -> Result<CircuitAst> {
    let content = std::fs::read_to_string(path).map_err(|e| crate::error::PedalerError::FileReadError {
        path: path.display().to_string(),
        source: e,
    })?;
    parse(&content)
}
