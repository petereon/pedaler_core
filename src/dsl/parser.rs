//! Parser for the circuit DSL.

use std::collections::HashMap;

use super::ast::*;
use super::lexer::{parse_value, Lexer, Token, TokenKind};
use crate::error::{PedalerError, Result};

/// Parser for circuit DSL.
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    peeked: Option<Token>,
}

impl<'a> Parser<'a> {
    /// Create a new parser with the given lexer.
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current = lexer.next_token().unwrap_or(Token {
            kind: TokenKind::Eof,
            text: String::new(),
            line: 1,
            column: 1,
        });
        Self {
            lexer,
            current,
            peeked: None,
        }
    }

    /// Parse the entire circuit description.
    pub fn parse(&mut self) -> Result<CircuitAst> {
        let mut ast = CircuitAst::new();
        let mut node_set = std::collections::HashSet::new();

        // Always include ground
        node_set.insert("0".to_string());
        node_set.insert("GND".to_string());

        while self.current.kind != TokenKind::Eof {
            // Skip empty lines
            if self.current.kind == TokenKind::Newline {
                self.advance()?;
                continue;
            }

            match &self.current.kind {
                TokenKind::Directive => {
                    self.parse_directive(&mut ast)?;
                }
                TokenKind::Identifier => {
                    let component = self.parse_component()?;
                    // Collect node names
                    for node in &component.nodes {
                        node_set.insert(node.clone());
                    }
                    ast.components.push(component);
                }
                TokenKind::Eof => break,
                _ => {
                    return Err(PedalerError::parse(
                        self.current.line,
                        format!("unexpected token: {:?}", self.current.text),
                    ));
                }
            }

            // Consume newline or EOF
            if self.current.kind == TokenKind::Newline {
                self.advance()?;
            }
        }

        // Build node list (excluding ground aliases)
        ast.nodes = node_set
            .into_iter()
            .filter(|n| n != "0" && n != "GND")
            .collect();

        Ok(ast)
    }

    fn advance(&mut self) -> Result<()> {
        self.current = if let Some(tok) = self.peeked.take() {
            tok
        } else {
            self.lexer.next_token()?
        };
        Ok(())
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token> {
        if self.current.kind == kind {
            let tok = self.current.clone();
            self.advance()?;
            Ok(tok)
        } else {
            Err(PedalerError::parse(
                self.current.line,
                format!("expected {:?}, got {:?}", kind, self.current.kind),
            ))
        }
    }

    fn parse_directive(&mut self, ast: &mut CircuitAst) -> Result<()> {
        let directive = self.current.text.clone();
        let line = self.current.line;
        self.advance()?;

        match directive.to_lowercase().as_str() {
            ".input" => {
                let node = self.expect(TokenKind::Identifier)?;
                ast.input_node = Some(node.text);
            }
            ".output" => {
                let node = self.expect(TokenKind::Identifier)?;
                ast.output_node = Some(node.text);
            }
            ".node" => {
                let node = self.expect(TokenKind::Identifier)?;
                if !ast.nodes.contains(&node.text) {
                    ast.nodes.push(node.text);
                }
            }
            ".model" => {
                let model = self.parse_model_def(line)?;
                if ast.models.contains_key(&model.name) {
                    return Err(PedalerError::DuplicateModel {
                        name: model.name,
                    });
                }
                ast.models.insert(model.name.clone(), model);
            }
            _ => {
                return Err(PedalerError::parse(
                    line,
                    format!("unknown directive: {}", directive),
                ));
            }
        }

        Ok(())
    }

    fn parse_model_def(&mut self, line: usize) -> Result<ModelDef> {
        let name = self.expect(TokenKind::Identifier)?.text;
        let type_str = self.expect(TokenKind::Identifier)?.text;

        let model_type = ModelType::from_str(&type_str).ok_or_else(|| {
            PedalerError::parse(line, format!("unknown model type: {}", type_str))
        })?;

        let mut params = HashMap::new();

        // Parse parameters in parentheses: (param=value param2=value2)
        if self.current.kind == TokenKind::OpenParen {
            self.advance()?;

            while self.current.kind != TokenKind::CloseParen
                && self.current.kind != TokenKind::Eof
                && self.current.kind != TokenKind::Newline
            {
                let param_name = self.expect(TokenKind::Identifier)?.text;
                self.expect(TokenKind::Equals)?;

                let value = if self.current.kind == TokenKind::Number {
                    let text = self.current.text.clone();
                    self.advance()?;
                    parse_value(&text).ok_or_else(|| {
                        PedalerError::parse(line, format!("invalid number: {}", text))
                    })?
                } else if self.current.kind == TokenKind::Identifier {
                    // Could be a number with unit like "1e-14"
                    let text = self.current.text.clone();
                    self.advance()?;
                    parse_value(&text).ok_or_else(|| {
                        PedalerError::parse(line, format!("invalid number: {}", text))
                    })?
                } else {
                    return Err(PedalerError::parse(line, "expected parameter value"));
                };

                params.insert(param_name.to_lowercase(), value);
            }

            if self.current.kind == TokenKind::CloseParen {
                self.advance()?;
            }
        }

        Ok(ModelDef {
            name,
            model_type,
            params,
            line,
        })
    }

    fn parse_component(&mut self) -> Result<ComponentDef> {
        let first_token = self.current.text.clone();
        let line = self.current.line;
        self.advance()?;

        // Determine component type from first token - check keywords FIRST before single-char prefix
        // This ensures REVERB isn't mistaken for a Resistor, DELAY for Diode, etc.
        let (component_type, name) = if let Some(ct) = ComponentType::from_keyword(&first_token) {
            // For keyword-based types (DELAY, REVERB, OPAMP, etc.), the NEXT token is the name
            let actual_name = self.expect(TokenKind::Identifier)?.text;
            (ct, actual_name)
        } else {
            // Check for multi-char prefixes
            let upper = first_token.to_uppercase();
            if upper.starts_with("OP") {
                (ComponentType::OpAmp, first_token)
            } else if upper.starts_with("POT") {
                (ComponentType::Potentiometer, first_token)
            } else if upper.starts_with("SW") {
                (ComponentType::Switch, first_token)
            } else if upper.starts_with("DELAY") {
                // DELAY keyword used as prefix - next token is name
                let actual_name = self.expect(TokenKind::Identifier)?.text;
                (ComponentType::Delay, actual_name)
            } else if upper.starts_with("REVERB") || upper.starts_with("REV") {
                // REVERB keyword used as prefix - next token is name
                let actual_name = self.expect(TokenKind::Identifier)?.text;
                (ComponentType::Reverb, actual_name)
            } else {
                // Finally, check single-character prefix
                let first_char = first_token.chars().next().unwrap_or('?');
                let ct = ComponentType::from_prefix(first_char).ok_or_else(|| {
                    PedalerError::UnknownComponentType {
                        component_type: first_token.clone(),
                        line,
                    }
                })?;
                (ct, first_token)
            }
        };

        let expected_nodes = component_type.expected_node_count();
        let mut nodes = Vec::with_capacity(expected_nodes);
        let mut value = None;
        let mut model_ref = None;
        let mut params = HashMap::new();

        // Parse nodes and optional parameters until end of line
        while self.current.kind != TokenKind::Newline && self.current.kind != TokenKind::Eof {
            match &self.current.kind {
                TokenKind::Identifier => {
                    let text = self.current.text.clone();
                    self.advance()?;

                    // Check for param=value syntax
                    if self.current.kind == TokenKind::Equals {
                        self.advance()?; // consume '='
                        // Parse the value
                        if self.current.kind == TokenKind::Number
                            || self.current.kind == TokenKind::Identifier
                        {
                            let val_text = self.current.text.clone();
                            self.advance()?;
                            if let Some(v) = parse_value(&val_text) {
                                params.insert(text.to_lowercase(), v);
                            }
                        }
                        continue;
                    }

                    // Check if this looks like a value with unit suffix
                    if nodes.len() >= expected_nodes {
                        if let Some(v) = parse_value(&text) {
                            value = Some(v);
                        } else {
                            // Could be a model reference
                            model_ref = Some(text);
                        }
                    } else {
                        // Check for special keywords first
                        if text == "DC" || text == "AC" {
                            // Source type keyword - next token should be value
                            params.insert(text.to_lowercase(), 1.0);
                            if self.current.kind == TokenKind::Number
                                || self.current.kind == TokenKind::Identifier
                            {
                                let val_text = self.current.text.clone();
                                self.advance()?;
                                if let Some(v) = parse_value(&val_text) {
                                    value = Some(v);
                                }
                            }
                        } else if text == "0" || text.to_uppercase() == "GND" {
                            nodes.push("0".to_string());
                        } else {
                            nodes.push(text);
                        }
                    }
                }
                TokenKind::Number => {
                    let text = self.current.text.clone();
                    self.advance()?;

                    if text == "0" && nodes.len() < expected_nodes {
                        // Ground node
                        nodes.push("0".to_string());
                    } else if let Some(v) = parse_value(&text) {
                        if value.is_none() {
                            value = Some(v);
                        } else {
                            // Additional numeric parameter (e.g., pot position)
                            params.insert("position".to_string(), v);
                        }
                    }
                }
                _ => break,
            }
        }

        // Validate node count
        if nodes.len() < expected_nodes {
            return Err(PedalerError::invalid_component(
                &name,
                line,
                format!(
                    "expected {} nodes, got {}",
                    expected_nodes,
                    nodes.len()
                ),
            ));
        }

        Ok(ComponentDef {
            component_type,
            name,
            nodes,
            value,
            model_ref,
            params,
            line,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_resistor() {
        let input = "R1 in out 10k";
        let ast = super::super::parse(input).unwrap();
        assert_eq!(ast.components.len(), 1);
        assert_eq!(ast.components[0].component_type, ComponentType::Resistor);
        assert_eq!(ast.components[0].name, "R1");
        assert_eq!(ast.components[0].nodes, vec!["in", "out"]);
        assert_eq!(ast.components[0].value, Some(10_000.0));
    }

    #[test]
    fn test_parse_input_output() {
        let input = ".input in\n.output out\nR1 in out 1k";
        let ast = super::super::parse(input).unwrap();
        assert_eq!(ast.input_node, Some("in".to_string()));
        assert_eq!(ast.output_node, Some("out".to_string()));
    }

    #[test]
    fn test_parse_model() {
        let input = ".model DCLIP D (is=1e-14 n=1.8)";
        let ast = super::super::parse(input).unwrap();
        assert!(ast.models.contains_key("DCLIP"));
        let model = &ast.models["DCLIP"];
        assert_eq!(model.model_type, ModelType::Diode);
        assert!((model.params["is"] - 1e-14).abs() < 1e-20);
    }

    #[test]
    fn test_parse_with_comments() {
        let input = "# This is a comment\nR1 in out 1k ; inline comment style\n";
        let ast = super::super::parse(input).unwrap();
        assert_eq!(ast.components.len(), 1);
    }
}
