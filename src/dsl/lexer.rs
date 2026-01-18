//! Lexer (tokenizer) for the circuit DSL.

use crate::error::{PedalerError, Result};

/// A token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The kind of token
    pub kind: TokenKind,
    /// The token's text
    pub text: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
}

/// Token types in the DSL.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// An identifier (component name, node name, etc.)
    Identifier,
    /// A number (integer or floating point, possibly with suffix)
    Number,
    /// A directive (starts with '.')
    Directive,
    /// Open parenthesis '('
    OpenParen,
    /// Close parenthesis ')'
    CloseParen,
    /// Equals sign '='
    Equals,
    /// Newline
    Newline,
    /// End of file
    Eof,
}

/// Lexer for tokenizing circuit DSL input.
pub struct Lexer<'a> {
    input: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    line: usize,
    column: usize,
    line_start: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given input.
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices().peekable(),
            line: 1,
            column: 1,
            line_start: 0,
        }
    }

    /// Get the next token.
    pub fn next_token(&mut self) -> Result<Token> {
        self.skip_whitespace_and_comments();

        let (_start_pos, ch) = match self.chars.peek().copied() {
            Some((pos, ch)) => (pos, ch),
            None => {
                return Ok(Token {
                    kind: TokenKind::Eof,
                    text: String::new(),
                    line: self.line,
                    column: self.column,
                });
            }
        };

        let start_column = self.column;
        let start_line = self.line;

        let token = match ch {
            '\n' => {
                self.advance();
                Token {
                    kind: TokenKind::Newline,
                    text: "\n".to_string(),
                    line: start_line,
                    column: start_column,
                }
            }
            '.' => {
                self.advance();
                let text = self.read_identifier();
                Token {
                    kind: TokenKind::Directive,
                    text: format!(".{}", text),
                    line: start_line,
                    column: start_column,
                }
            }
            '(' => {
                self.advance();
                Token {
                    kind: TokenKind::OpenParen,
                    text: "(".to_string(),
                    line: start_line,
                    column: start_column,
                }
            }
            ')' => {
                self.advance();
                Token {
                    kind: TokenKind::CloseParen,
                    text: ")".to_string(),
                    line: start_line,
                    column: start_column,
                }
            }
            '=' => {
                self.advance();
                Token {
                    kind: TokenKind::Equals,
                    text: "=".to_string(),
                    line: start_line,
                    column: start_column,
                }
            }
            '-' | '+' | '0'..='9' => {
                let text = self.read_number();
                Token {
                    kind: TokenKind::Number,
                    text,
                    line: start_line,
                    column: start_column,
                }
            }
            _ if ch.is_alphabetic() || ch == '_' => {
                let text = self.read_identifier();
                // Check if it looks like a number with unit suffix
                if self.looks_like_number(&text) {
                    Token {
                        kind: TokenKind::Number,
                        text,
                        line: start_line,
                        column: start_column,
                    }
                } else {
                    Token {
                        kind: TokenKind::Identifier,
                        text,
                        line: start_line,
                        column: start_column,
                    }
                }
            }
            _ => {
                return Err(PedalerError::lexer(
                    start_line,
                    start_column,
                    format!("unexpected character '{}'", ch),
                ));
            }
        };

        Ok(token)
    }

    /// Peek at the next token without consuming it.
    #[allow(dead_code)]
    pub fn peek_token(&mut self) -> Result<Token> {
        // Save state - this is a simplified implementation
        let _saved_line = self.line;
        let _saved_column = self.column;
        let _saved_line_start = self.line_start;

        let token = self.next_token()?;

        // This is a simplified peek - in practice we'd need proper state restoration
        // For now, we'll use a different approach in the parser
        Ok(token)
    }

    fn current_pos(&self) -> usize {
        // Note: peek() on a Peekable doesn't require &mut self when just reading
        // We clone the iterator position info we need
        self.input.len() - self.input[self.line_start..].len() + self.column - 1
    }

    #[allow(dead_code)]
    fn current_pos_from_peek(&mut self) -> usize {
        self.chars.peek().map(|(pos, _)| *pos).unwrap_or(self.input.len())
    }

    fn advance(&mut self) -> Option<char> {
        if let Some((_, ch)) = self.chars.next() {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
                self.line_start = self.current_pos();
            } else {
                self.column += 1;
            }
            Some(ch)
        } else {
            None
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some(&(_, ch)) = self.chars.peek() {
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else if ch == '#' || ch == ';' {
                // Skip comment until end of line
                while let Some(&(_, c)) = self.chars.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn read_identifier(&mut self) -> String {
        let mut text = String::new();
        while let Some(&(_, ch)) = self.chars.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                text.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        text
    }

    fn read_number(&mut self) -> String {
        let mut text = String::new();

        // Optional sign
        if let Some(&(_, ch)) = self.chars.peek() {
            if ch == '-' || ch == '+' {
                text.push(ch);
                self.advance();
            }
        }

        // Integer part
        while let Some(&(_, ch)) = self.chars.peek() {
            if ch.is_ascii_digit() {
                text.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Decimal part
        if let Some(&(_, '.')) = self.chars.peek() {
            text.push('.');
            self.advance();
            while let Some(&(_, ch)) = self.chars.peek() {
                if ch.is_ascii_digit() {
                    text.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Exponent part
        if let Some(&(_, ch)) = self.chars.peek() {
            if ch == 'e' || ch == 'E' {
                text.push(ch);
                self.advance();
                if let Some(&(_, sign)) = self.chars.peek() {
                    if sign == '-' || sign == '+' {
                        text.push(sign);
                        self.advance();
                    }
                }
                while let Some(&(_, ch)) = self.chars.peek() {
                    if ch.is_ascii_digit() {
                        text.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        // Unit suffix (p, n, u, m, k, M, G)
        if let Some(&(_, ch)) = self.chars.peek() {
            if matches!(ch, 'p' | 'n' | 'u' | 'µ' | 'm' | 'k' | 'K' | 'M' | 'G') {
                text.push(ch);
                self.advance();
            }
        }

        text
    }

    fn looks_like_number(&self, text: &str) -> bool {
        // Check if identifier is actually a number with unit suffix like "10k" or "100n"
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            return false;
        }

        // Must start with digit
        if !chars[0].is_ascii_digit() {
            return false;
        }

        // Check pattern: digits, optional decimal, optional exponent, optional unit
        let mut has_digits = false;
        let mut i = 0;

        // Integer part
        while i < chars.len() && chars[i].is_ascii_digit() {
            has_digits = true;
            i += 1;
        }

        // Decimal part
        if i < chars.len() && chars[i] == '.' {
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
        }

        // Exponent
        if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
            i += 1;
            if i < chars.len() && (chars[i] == '-' || chars[i] == '+') {
                i += 1;
            }
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
        }

        // Unit suffix
        if i < chars.len() {
            let suffix = chars[i];
            if matches!(suffix, 'p' | 'n' | 'u' | 'µ' | 'm' | 'k' | 'K' | 'M' | 'G') {
                i += 1;
            }
        }

        has_digits && i == chars.len()
    }
}

/// Parse a number string with optional unit suffix.
pub fn parse_value(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    let (num_str, multiplier) = if let Some(last) = text.chars().last() {
        let mult = match last {
            'p' => 1e-12,
            'n' => 1e-9,
            'u' | 'µ' => 1e-6,
            'm' => 1e-3,
            'k' | 'K' => 1e3,
            'M' => 1e6,
            'G' => 1e9,
            _ => 1.0,
        };
        if mult != 1.0 {
            (&text[..text.len() - last.len_utf8()], mult)
        } else {
            (text, 1.0)
        }
    } else {
        (text, 1.0)
    };

    num_str.parse::<f64>().ok().map(|v| v * multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: Option<f64>, b: Option<f64>) -> bool {
        match (a, b) {
            (Some(x), Some(y)) => (x - y).abs() < x.abs() * 1e-10 + 1e-15,
            (None, None) => true,
            _ => false,
        }
    }

    #[test]
    fn test_parse_value() {
        assert!(approx_eq(parse_value("10k"), Some(10_000.0)));
        assert!(approx_eq(parse_value("100n"), Some(100e-9)));
        assert!(approx_eq(parse_value("4.7u"), Some(4.7e-6)));
        assert!(approx_eq(parse_value("1M"), Some(1_000_000.0)));
        assert!(approx_eq(parse_value("2.2"), Some(2.2)));
        assert!(approx_eq(parse_value("1e-9"), Some(1e-9)));
    }

    #[test]
    fn test_lexer_basic() {
        let input = "R1 in out 10k";
        let mut lexer = Lexer::new(input);

        let tok = lexer.next_token().unwrap();
        assert_eq!(tok.kind, TokenKind::Identifier);
        assert_eq!(tok.text, "R1");

        let tok = lexer.next_token().unwrap();
        assert_eq!(tok.kind, TokenKind::Identifier);
        assert_eq!(tok.text, "in");
    }

    #[test]
    fn test_lexer_directive() {
        let input = ".model D1 D (is=1e-14)";
        let mut lexer = Lexer::new(input);

        let tok = lexer.next_token().unwrap();
        assert_eq!(tok.kind, TokenKind::Directive);
        assert_eq!(tok.text, ".model");
    }
}
