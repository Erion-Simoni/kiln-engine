//! Lexer for KilnScript.

use crate::diagnostic::Span;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Number(i64),
    Boolean(bool),
    Float(f64),
    String(String),
    Identifier(String),

    // Keywords
    If,
    Else,
    Return,
    Call,
    Let,
    Fn,
    While,
    For,
    Enum,
    Break,
    Continue,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// `=` — assignment to an existing variable.
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    And,
    Or,
    Not,
    /// Invalid token retained by the parser for lexical diagnostics.
    Invalid(String),

    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Dot,
    Semicolon,

    // End of input
    Eof,
}

#[derive(Debug, Clone)]
pub struct TokenWithSpan {
    pub token: Token,
    pub span: Span,
}

pub struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
    pub(crate) pos: usize,
    pub(crate) token_start: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.chars().peekable(),
            pos: 0,
            token_start: 0,
        }
    }

    pub fn next_token(&mut self) -> Result<TokenWithSpan, String> {
        self.skip_whitespace();

        let start = self.pos;
        self.token_start = start;

        match self.peek_char() {
            None => Ok(TokenWithSpan::new(Token::Eof, Span::new(start, start))),
            Some(c) => {
                if c.is_ascii_digit() {
                    return self.read_number(start);
                }
                if c.is_ascii_alphabetic() || c == '_' {
                    return self.read_identifier(start);
                }
                if c == '"' {
                    return self.read_string(start);
                }
                if let Some(token) = self.read_operator_or_punct() {
                    return Ok(TokenWithSpan::new(token, Span::new(start, self.pos)));
                }
                Err(format!("Unexpected character: {}", c))
            }
        }
    }

    fn skip_whitespace(&mut self) {
        loop {
            while let Some(&c) = self.input.peek() {
                if c.is_ascii_whitespace() {
                    self.input.next();
                    self.pos += 1;
                } else {
                    break;
                }
            }

            // Line comments run to the end of the line.
            if self.input.peek() == Some(&'/') && self.input.clone().nth(1) == Some('/') {
                for c in self.input.by_ref() {
                    self.pos += c.len_utf8();
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }

            break;
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.input.peek().copied()
    }

    fn next_char(&mut self) -> Option<char> {
        let c = self.input.next();
        if let Some(c) = c {
            self.pos += c.len_utf8();
        }
        c
    }

    fn read_number(&mut self, start: usize) -> Result<TokenWithSpan, String> {
        let mut num_str = String::new();

        while let Some(&c) = self.input.peek() {
            if c.is_ascii_digit() {
                num_str.push(c);
                self.next_char();
            } else {
                break;
            }
        }

        // Fractional part: "1.5" → Float. A trailing '.' with no digits
        // (e.g. "1..2") is left for the caller to reject as punctuation.
        let is_float = self.input.peek() == Some(&'.')
            && self
                .input
                .clone()
                .nth(1)
                .is_some_and(|c| c.is_ascii_digit());
        if is_float {
            num_str.push(self.next_char().unwrap()); // '.'
            while let Some(&c) = self.input.peek() {
                if c.is_ascii_digit() {
                    num_str.push(c);
                    self.next_char();
                } else {
                    break;
                }
            }
            let value = num_str
                .parse::<f64>()
                .map_err(|_| format!("Invalid number: {}", num_str))?;
            return Ok(TokenWithSpan::new(
                Token::Float(value),
                Span::new(start, self.pos),
            ));
        }

        let value = num_str
            .parse::<i64>()
            .map_err(|_| format!("Invalid number: {}", num_str))?;

        Ok(TokenWithSpan::new(
            Token::Number(value),
            Span::new(start, self.pos),
        ))
    }

    fn read_identifier(&mut self, start: usize) -> Result<TokenWithSpan, String> {
        let mut ident = String::new();

        while let Some(&c) = self.input.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                ident.push(c);
                self.next_char();
            } else {
                break;
            }
        }

        let token = match ident.as_str() {
            "true" => Token::Boolean(true),
            "false" => Token::Boolean(false),
            "if" => Token::If,
            "else" => Token::Else,
            "return" => Token::Return,
            "call" => Token::Call,
            "let" => Token::Let,
            "fn" => Token::Fn,
            "while" => Token::While,
            "for" => Token::For,
            "enum" => Token::Enum,
            "break" => Token::Break,
            "continue" => Token::Continue,
            _ => Token::Identifier(ident),
        };

        Ok(TokenWithSpan::new(token, Span::new(start, self.pos)))
    }

    fn read_string(&mut self, start: usize) -> Result<TokenWithSpan, String> {
        // Consume the opening quote.
        self.next_char();

        let mut s = String::new();
        while let Some(c) = self.peek_char() {
            if c == '"' {
                self.next_char(); // consume closing quote
                return Ok(TokenWithSpan::new(
                    Token::String(s),
                    Span::new(start, self.pos),
                ));
            }
            if c == '\\' {
                self.next_char(); // consume backslash
                match self.next_char() {
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some('n') => s.push('\n'),
                    Some('r') => s.push('\r'),
                    Some('t') => s.push('\t'),
                    Some(c) => s.push(c),
                    None => return Err("Unterminated string escape".to_string()),
                }
                continue;
            }
            if c.is_ascii_control() {
                return Err("Control character in string".to_string());
            }
            s.push(c);
            self.next_char();
        }
        Err("Unterminated string literal".to_string())
    }

    fn read_operator_or_punct(&mut self) -> Option<Token> {
        let c = self.next_char()?;

        Some(match c {
            '+' => {
                if self.peek_char() == Some('=') {
                    self.next_char();
                    Token::PlusAssign
                } else {
                    Token::Plus
                }
            }
            '-' => {
                if self.peek_char() == Some('=') {
                    self.next_char();
                    Token::MinusAssign
                } else {
                    Token::Minus
                }
            }
            '*' => {
                if self.peek_char() == Some('=') {
                    self.next_char();
                    Token::StarAssign
                } else {
                    Token::Star
                }
            }
            '/' => {
                if self.peek_char() == Some('=') {
                    self.next_char();
                    Token::SlashAssign
                } else {
                    Token::Slash
                }
            }
            '=' => {
                if self.peek_char() == Some('=') {
                    self.next_char();
                    Token::EqEq
                } else {
                    Token::Assign
                }
            }
            '&' if self.peek_char() == Some('&') => {
                self.next_char();
                Token::And
            }
            '|' if self.peek_char() == Some('|') => {
                self.next_char();
                Token::Or
            }
            '!' => {
                if self.peek_char() == Some('=') {
                    self.next_char();
                    Token::Ne
                } else {
                    Token::Not
                }
            }
            '<' => {
                if self.peek_char() == Some('=') {
                    self.next_char();
                    Token::Le
                } else {
                    Token::Lt
                }
            }
            '>' => {
                if self.peek_char() == Some('=') {
                    self.next_char();
                    Token::Ge
                } else {
                    Token::Gt
                }
            }
            '(' => Token::LParen,
            ')' => Token::RParen,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            ',' => Token::Comma,
            '.' => Token::Dot,
            ';' => Token::Semicolon,
            _ => return None,
        })
    }
}

impl TokenWithSpan {
    pub fn new(token: Token, span: Span) -> Self {
        Self { token, span }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_numbers() {
        let mut lexer = Lexer::new("42 100");
        let t1 = lexer.next_token().unwrap();
        assert_eq!(t1.token, Token::Number(42));
        let t2 = lexer.next_token().unwrap();
        assert_eq!(t2.token, Token::Number(100));
    }

    #[test]
    fn lex_identifiers() {
        let mut lexer = Lexer::new("foo bar_baz");
        let t1 = lexer.next_token().unwrap();
        assert_eq!(t1.token, Token::Identifier("foo".to_string()));
        let t2 = lexer.next_token().unwrap();
        assert_eq!(t2.token, Token::Identifier("bar_baz".to_string()));
    }

    #[test]
    fn lex_keywords() {
        let mut lexer = Lexer::new("if else return call");
        assert_eq!(lexer.next_token().unwrap().token, Token::If);
        assert_eq!(lexer.next_token().unwrap().token, Token::Else);
        assert_eq!(lexer.next_token().unwrap().token, Token::Return);
        assert_eq!(lexer.next_token().unwrap().token, Token::Call);
    }

    #[test]
    fn lex_operators() {
        let mut lexer = Lexer::new("+-*/ == != < <= > >= ");
        assert_eq!(lexer.next_token().unwrap().token, Token::Plus);
        assert_eq!(lexer.next_token().unwrap().token, Token::Minus);
        assert_eq!(lexer.next_token().unwrap().token, Token::Star);
        assert_eq!(lexer.next_token().unwrap().token, Token::Slash);
        assert_eq!(lexer.next_token().unwrap().token, Token::EqEq);
        assert_eq!(lexer.next_token().unwrap().token, Token::Ne);
        assert_eq!(lexer.next_token().unwrap().token, Token::Lt);
        assert_eq!(lexer.next_token().unwrap().token, Token::Le);
        assert_eq!(lexer.next_token().unwrap().token, Token::Gt);
        assert_eq!(lexer.next_token().unwrap().token, Token::Ge);
    }

    #[test]
    fn lex_whitespace() {
        let mut lexer = Lexer::new("  42  ");
        let t = lexer.next_token().unwrap();
        assert_eq!(t.token, Token::Number(42));
    }

    #[test]
    fn lex_line_comment() {
        let mut lexer = Lexer::new("// a comment\n42 // trailing\n7");
        assert_eq!(lexer.next_token().unwrap().token, Token::Number(42));
        assert_eq!(lexer.next_token().unwrap().token, Token::Number(7));
    }

    #[test]
    fn lex_float_literal() {
        let mut lexer = Lexer::new("1.25 2.5 42");
        assert_eq!(lexer.next_token().unwrap().token, Token::Float(1.25));
        assert_eq!(lexer.next_token().unwrap().token, Token::Float(2.5));
        assert_eq!(lexer.next_token().unwrap().token, Token::Number(42));
    }

    #[test]
    fn lex_string_literal() {
        let mut lexer = Lexer::new(r#""hello" "with \"quotes\"""#);
        assert_eq!(
            lexer.next_token().unwrap().token,
            Token::String("hello".to_string())
        );
        assert_eq!(
            lexer.next_token().unwrap().token,
            Token::String("with \"quotes\"".to_string())
        );
    }

    #[test]
    fn lex_unterminated_string_errors() {
        let mut lexer = Lexer::new("\"oops");
        assert!(lexer.next_token().is_err());
    }
}
