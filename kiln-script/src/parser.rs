//! Parser for KilnScript.

use crate::diagnostic::{Diagnostic, Span};
use crate::lexer::{Lexer, Token, TokenWithSpan};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Parse error: {0}")]
    Custom(String),
    #[error("Unexpected token: {0:?}")]
    UnexpectedToken(Token),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(i64, crate::diagnostic::Span),
    Identifier(String, crate::diagnostic::Span),
    BinaryOp {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        func: String,
        args: Vec<Expr>,
        span: crate::diagnostic::Span,
    },
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Return(Expr),
    If {
        condition: Expr,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
    },
}

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Option<TokenWithSpan>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer::new(input);
        let current = lexer.next_token().ok();
        Self {
            lexer,
            current,
            diagnostics: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();

        while !self.is_at_end() {
            if let Some(stmt) = self.parse_stmt()? {
                stmts.push(stmt);
            }
        }

        Ok(stmts)
    }

    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    fn parse_stmt(&mut self) -> Result<Option<Stmt>, ParseError> {
        match self.peek_token() {
            Some(Token::If) => self.parse_if(),
            Some(Token::Return) => self.parse_return(),
            Some(Token::Semicolon) => {
                self.advance();
                Ok(None)
            }
            _ => {
                let expr = self.parse_expr()?;
                if self.peek_token() == Some(Token::Semicolon) {
                    self.advance();
                }
                Ok(Some(Stmt::Expr(expr)))
            }
        }
    }

    fn parse_if(&mut self) -> Result<Option<Stmt>, ParseError> {
        self.consume(Token::If)?;
        let condition = self.parse_expr()?;
        self.consume(Token::LBrace)?;
        let then_block = self.parse_block()?;
        self.consume(Token::RBrace)?;

        let else_block = if self.peek_token() == Some(Token::Else) {
            self.advance();
            self.consume(Token::LBrace)?;
            let block = self.parse_block()?;
            self.consume(Token::RBrace)?;
            Some(block)
        } else {
            None
        };

        Ok(Some(Stmt::If {
            condition,
            then_block,
            else_block,
        }))
    }

    fn parse_return(&mut self) -> Result<Option<Stmt>, ParseError> {
        self.consume(Token::Return)?;
        let expr = self.parse_expr()?;
        if self.peek_token() == Some(Token::Semicolon) {
            self.advance();
        }
        Ok(Some(Stmt::Return(expr)))
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();
        while self.peek_token() != Some(Token::RBrace) && !self.is_at_end() {
            if let Some(stmt) = self.parse_stmt()? {
                stmts.push(stmt);
            }
        }
        Ok(stmts)
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_term()?;

        while let Some(op) = self.match_comparison_op() {
            let right = self.parse_term()?;
            expr = Expr::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_factor()?;

        while let Some(op) = self.match_additive_op() {
            let right = self.parse_factor()?;
            expr = Expr::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        while let Some(op) = self.match_multiplicative_op() {
            let right = self.parse_primary()?;
            expr = Expr::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.peek_token() {
            Some(Token::Number(n)) => {
                let span = self.current.as_ref().map(|t| t.span).unwrap();
                self.advance();
                Ok(Expr::Number(n, span))
            }
            Some(Token::Identifier(name)) => {
                let span = self.current.as_ref().map(|t| t.span).unwrap();
                self.advance();
                if self.peek_token() == Some(Token::LParen) {
                    self.parse_call(name.clone(), span)
                } else {
                    Ok(Expr::Identifier(name.clone(), span))
                }
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expr()?;
                self.consume(Token::RParen)?;
                Ok(expr)
            }
            Some(token) => {
                let span = self.current.as_ref().map(|t| t.span).unwrap();
                self.diagnostics.push(Diagnostic::new(span, format!("Unexpected token: {:?}", token)));
                Err(ParseError::UnexpectedToken(token))
            }
            None => Err(ParseError::Custom("Unexpected end of input".to_string())),
        }
    }

    fn parse_call(&mut self, func: String, span: crate::diagnostic::Span) -> Result<Expr, ParseError> {
        self.consume(Token::LParen)?;
        let mut args = Vec::new();

        if self.peek_token() != Some(Token::RParen) {
            loop {
                args.push(self.parse_expr()?);
                if self.peek_token() != Some(Token::Comma) {
                    break;
                }
                self.advance();
            }
        }

        self.consume(Token::RParen)?;
        Ok(Expr::Call { func, args, span })
    }

    fn match_comparison_op(&mut self) -> Option<BinaryOp> {
        let op = match self.peek_token() {
            Some(Token::EqEq) => Some(BinaryOp::Eq),
            Some(Token::Ne) => Some(BinaryOp::Ne),
            Some(Token::Lt) => Some(BinaryOp::Lt),
            Some(Token::Le) => Some(BinaryOp::Le),
            Some(Token::Gt) => Some(BinaryOp::Gt),
            Some(Token::Ge) => Some(BinaryOp::Ge),
            _ => None,
        };

        if op.is_some() {
            self.advance();
        }

        op
    }

    fn match_additive_op(&mut self) -> Option<BinaryOp> {
        let op = match self.peek_token() {
            Some(Token::Plus) => Some(BinaryOp::Add),
            Some(Token::Minus) => Some(BinaryOp::Sub),
            _ => None,
        };

        if op.is_some() {
            self.advance();
        }

        op
    }

    fn match_multiplicative_op(&mut self) -> Option<BinaryOp> {
        let op = match self.peek_token() {
            Some(Token::Star) => Some(BinaryOp::Mul),
            Some(Token::Slash) => Some(BinaryOp::Div),
            _ => None,
        };

        if op.is_some() {
            self.advance();
        }

        op
    }

    fn consume(&mut self, expected: Token) -> Result<(), ParseError> {
        if self.peek_token() == Some(expected.clone()) {
            self.advance();
            Ok(())
        } else {
            let span = self.current.as_ref().map(|t| t.span).unwrap_or(Span::new(0, 0));
            self.diagnostics.push(Diagnostic::new(
                span,
                format!(
                    "Expected {:?}, found {:?}",
                    expected,
                    self.current.as_ref().map(|t| &t.token).unwrap_or(&Token::Eof)
                ),
            ));
            Err(ParseError::Custom(format!(
                "Expected {:?}, found {:?}",
                expected,
                self.current.as_ref().map(|t| &t.token).unwrap_or(&Token::Eof)
            )))
        }
    }

    fn advance(&mut self) {
        self.current = self.lexer.next_token().ok();
    }

    fn peek_token(&self) -> Option<Token> {
        self.current.as_ref().map(|t| t.token.clone())
    }

    fn is_at_end(&self) -> bool {
        self.peek_token() == Some(Token::Eof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_number() {
        let mut parser = Parser::new("42");
        let stmts = parser.parse().unwrap();
        assert_eq!(stmts.len(), 1);
        assert!(matches!(stmts[0], Stmt::Expr(Expr::Number(42, _))));
    }

    #[test]
    fn parse_binary_expr() {
        let mut parser = Parser::new("1 + 2");
        let stmts = parser.parse().unwrap();
        assert_eq!(stmts.len(), 1);
        if let Stmt::Expr(Expr::BinaryOp { op, .. }) = &stmts[0] {
            assert!(matches!(op, BinaryOp::Add));
        } else {
            panic!("Expected binary op");
        }
    }

    #[test]
    fn parse_if_stmt() {
        let mut parser = Parser::new("if true { 42 }");
        let stmts = parser.parse().unwrap();
        assert_eq!(stmts.len(), 1);
        assert!(matches!(stmts[0], Stmt::If { .. }));
    }

    #[test]
    fn parse_call() {
        let mut parser = Parser::new("foo()");
        let stmts = parser.parse().unwrap();
        assert_eq!(stmts.len(), 1);
        assert!(matches!(stmts[0], Stmt::Expr(Expr::Call { .. })));
    }
}
