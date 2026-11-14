//! Lossless syntax colors for complete or unfinished KilnScript buffers.
use egui::{text::LayoutJob, Color32, FontId, TextFormat};
use kiln_runtime::slice::BUILTINS;

pub(crate) fn highlight(source: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut start = 0;
    while start < source.len() {
        let tail = &source[start..];
        let first = tail.chars().next().expect("nonempty suffix");
        let (len, color) = if tail.starts_with("//") {
            (
                tail.find('\n').unwrap_or(tail.len()),
                Color32::from_rgb(120, 145, 120),
            )
        } else if first == '"' {
            let mut chars = tail.char_indices().skip(1);
            let mut end = tail.len();
            while let Some((offset, ch)) = chars.next() {
                if ch == '\\' {
                    chars.next();
                } else if ch == '"' {
                    end = offset + 1;
                    break;
                }
            }
            (end, Color32::from_rgb(220, 185, 125))
        } else if first.is_ascii_alphabetic() || first == '_' {
            let len = tail
                .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
                .unwrap_or(tail.len());
            let word = &tail[..len];
            let color = if [
                "let", "fn", "if", "else", "return", "call", "while", "for", "break", "continue",
                "true", "false", "import",
            ]
            .contains(&word)
            {
                Color32::from_rgb(190, 150, 240)
            } else if BUILTINS.iter().any(|(name, _, _)| *name == word) {
                Color32::from_rgb(115, 195, 230)
            } else {
                Color32::from_gray(220)
            };
            (len, color)
        } else if first.is_ascii_digit() {
            (
                tail.find(|ch: char| !ch.is_ascii_digit() && ch != '.')
                    .unwrap_or(tail.len()),
                Color32::from_rgb(165, 210, 155),
            )
        } else {
            (first.len_utf8(), Color32::from_gray(220))
        };
        job.append(
            &tail[..len],
            0.0,
            TextFormat {
                font_id: FontId::monospace(14.0),
                color,
                ..Default::default()
            },
        );
        start += len;
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn colors_preserve_unicode_comments_escaped_quotes_and_unfinished_text() {
        for code in [
            "// café 🦀\nfor(let i=0;i<3;i+=1) {print(\"héllo\");}",
            r#"let x="escaped \" quote";"#,
            "// no final newline",
            "let incomplete=\"unfinished 🦀",
            "",
            "☃ @",
        ] {
            let job = highlight(code);
            assert_eq!(job.text, code);
            for section in &job.sections {
                assert!(code.get(section.byte_range.clone()).is_some());
            }
        }
        let job = highlight("for print 42");
        assert_ne!(job.sections[0].format.color, job.sections[2].format.color);
    }
}
