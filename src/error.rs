//! Error types for the Pedaler circuit simulator.
//!
//! This module provides a unified error type [`PedalerError`] that covers
//! all error conditions that can occur during DSL parsing, circuit validation,
//! and simulation.

use thiserror::Error;

/// Result type alias using [`PedalerError`].
pub type Result<T> = std::result::Result<T, PedalerError>;

/// Unified error type for all Pedaler operations.
#[derive(Error, Debug)]
pub enum PedalerError {
    // ============ DSL Parsing Errors ============
    /// Error during lexical analysis
    #[error("Lexer error at line {line}, column {column}: {message}")]
    LexerError {
        line: usize,
        column: usize,
        message: String,
    },

    /// Error during parsing
    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },

    /// Invalid component definition
    #[error("Invalid component '{name}' at line {line}: {message}")]
    InvalidComponent {
        name: String,
        line: usize,
        message: String,
    },

    /// Unknown component type
    #[error("Unknown component type '{component_type}' at line {line}")]
    UnknownComponentType { component_type: String, line: usize },

    /// Invalid parameter value
    #[error("Invalid parameter '{param}' for component '{component}': {message}")]
    InvalidParameter {
        component: String,
        param: String,
        message: String,
    },

    /// Undefined model reference
    #[error("Undefined model '{model}' referenced by component '{component}'")]
    UndefinedModel { model: String, component: String },

    // ============ Circuit Validation Errors ============
    /// Node not found in circuit
    #[error("Node '{node}' not found in circuit")]
    NodeNotFound { node: String },

    /// Floating node (not connected to ground path)
    #[error("Floating node '{node}' detected - no path to ground")]
    FloatingNode { node: String },

    /// Missing ground node
    #[error("Circuit has no ground node (use '0' or 'GND')")]
    MissingGround,

    /// Missing input node
    #[error("No input node specified (use '.input <node>')")]
    MissingInput,

    /// Missing output node
    #[error("No output node specified (use '.output <node>')")]
    MissingOutput,

    /// Duplicate component name
    #[error("Duplicate component name '{name}'")]
    DuplicateComponent { name: String },

    /// Duplicate model name
    #[error("Duplicate model name '{name}'")]
    DuplicateModel { name: String },

    /// Invalid circuit topology
    #[error("Invalid circuit topology: {message}")]
    InvalidTopology { message: String },

    // ============ Simulation Errors ============
    /// Matrix is singular and cannot be solved
    #[error("Singular matrix - circuit may have a short circuit or floating node")]
    SingularMatrix,

    /// Newton-Raphson iteration did not converge
    #[error("Newton-Raphson did not converge after {iterations} iterations (residual: {residual:.2e})")]
    ConvergenceFailure { iterations: usize, residual: f64 },

    /// Numerical overflow detected
    #[error("Numerical overflow detected at node '{node}' (value: {value:.2e})")]
    NumericalOverflow { node: String, value: f64 },

    /// Invalid simulation parameter
    #[error("Invalid simulation parameter: {message}")]
    InvalidSimulationParam { message: String },

    // ============ I/O Errors ============
    /// Error reading circuit file
    #[error("Failed to read circuit file '{path}': {source}")]
    FileReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Error reading audio input
    #[error("Audio input error: {message}")]
    AudioInputError { message: String },

    /// Error writing audio output
    #[error("Audio output error: {message}")]
    AudioOutputError { message: String },

    // ============ WASM Errors ============
    /// WASM-specific error
    #[cfg(feature = "wasm")]
    #[error("WASM error: {message}")]
    WasmError { message: String },
}

impl PedalerError {
    /// Create a lexer error
    pub fn lexer(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self::LexerError {
            line,
            column,
            message: message.into(),
        }
    }

    /// Create a parse error
    pub fn parse(line: usize, message: impl Into<String>) -> Self {
        Self::ParseError {
            line,
            message: message.into(),
        }
    }

    /// Create an invalid component error
    pub fn invalid_component(name: impl Into<String>, line: usize, message: impl Into<String>) -> Self {
        Self::InvalidComponent {
            name: name.into(),
            line,
            message: message.into(),
        }
    }

    /// Create a convergence failure error
    pub fn convergence_failure(iterations: usize, residual: f64) -> Self {
        Self::ConvergenceFailure {
            iterations,
            residual,
        }
    }
}
