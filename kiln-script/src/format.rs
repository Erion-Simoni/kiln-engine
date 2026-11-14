//! Whitespace-only indentation formatting, preserving comments and string contents.
use crate::lexer::{Lexer, Token};

/// Format balanced source without changing tokens, comments, or valid string contents.
/// Imported resources need not be opened; declarations can remain unfinished.
pub fn format_source(source: &str) -> Result<String, String> {
    if source.len() > 1_048_576 {
        return Err("Source exceeds 1 MiB".into());
    }
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();
    let mut stack = Vec::new();
    let mut strings = Vec::new();
    loop {
        let token = lexer.next_token()?;
        match &token.token {
            Token::LBrace => stack.push('}'),
            Token::LParen => stack.push(')'),
            Token::RBrace if stack.pop() != Some('}') => return Err("Unbalanced braces".into()),
            Token::RParen if stack.pop() != Some(')') => {
                return Err("Unbalanced parentheses".into())
            }
            Token::String(_) => strings.push(token.span),
            Token::Eof => break,
            _ => {}
        }
        if stack.len() > 256 {
            return Err("Formatting nesting exceeds 256".into());
        }
        tokens.push(token);
    }
    if !stack.is_empty() {
        return Err("Source has an unclosed delimiter".into());
    }
    let inside = |offset: usize| {
        let i = strings.partition_point(|span| span.end <= offset);
        strings
            .get(i)
            .is_some_and(|span| span.start < offset && offset < span.end)
    };
    let mut depth = 0_usize;
    let mut start = 0;
    let mut cursor = 0;
    let mut formatted = String::new();
    for raw in source.split_inclusive('\n') {
        let line = raw.strip_suffix('\n').unwrap_or(raw);
        let end = start + line.len();
        let first = cursor;
        while cursor < tokens.len() && tokens[cursor].span.start < start + raw.len() {
            cursor += 1;
        }
        let line_tokens = &tokens[first..cursor];
        let closing = line_tokens
            .iter()
            .take_while(|token| matches!(token.token, Token::RBrace))
            .count();
        if !inside(start) && !line.trim().is_empty() {
            formatted.push_str(&"    ".repeat(depth.saturating_sub(closing)));
        }
        let content = if inside(start) {
            line
        } else {
            line.trim_start()
        };
        formatted.push_str(if inside(end) {
            content
        } else {
            content.trim_end()
        });
        formatted.push('\n');
        for token in line_tokens {
            match token.token {
                Token::LBrace => depth += 1,
                Token::RBrace => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        if formatted.len() > 1_048_576 {
            return Err("Formatted source would exceed 1 MiB".into());
        }
        start += raw.len();
    }
    Ok(formatted)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tokens(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source);
        let mut result = Vec::new();
        loop {
            let t = lexer.next_token().unwrap().token;
            if t == Token::Eof {
                break;
            }
            result.push(t);
        }
        result
    }
    #[test]
    fn formatting_is_idempotent_and_preserves_imports_comments_unicode_and_escaped_strings() {
        let code = r#"import "enemy.ks";
// café { comment
if true {
let text="keep   \n   🦀 } still text  ";
 if false {
return text;
}
}
"#;
        let output = format_source(code).unwrap();
        assert_eq!(tokens(code), tokens(&output));
        assert_eq!(format_source(&output).unwrap(), output);
        assert!(output.contains("    let text="));
        assert!(output.contains("   🦀 } still text  \""));
        assert!(output.contains("// café { comment"));
    }
    #[test]
    fn invalid_quoted_unbalanced_and_excessively_nested_buffers_are_not_rewritten() {
        for code in ["if true {", "}", "let s=\"unterminated", "((1)", "{)"] {
            assert!(format_source(code).is_err(), "{code}");
        }
        assert!(format_source(&format!("{}1{}", "(".repeat(257), ")".repeat(257))).is_err());
        assert_eq!(format_source("").unwrap(), "");
    }
}
