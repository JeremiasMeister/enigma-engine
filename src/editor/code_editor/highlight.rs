use egui::{text::LayoutJob, Color32, FontId, TextFormat};

#[derive(Default)]
pub struct Highlight {
    text: String,
    rust: bool,
    job: LayoutJob,
}

impl Highlight {
    pub fn layout(&mut self, text: &str, rust: bool) -> LayoutJob {
        if self.text != text || self.rust != rust {
            self.text = text.into();
            self.rust = rust;
            self.job = highlight(text, rust);
        }
        self.job.clone()
    }
}

fn highlight(text: &str, rust: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    let plain = Color32::from_rgb(218, 227, 240);
    let append = |job: &mut LayoutJob, part: &str, color| {
        job.append(
            part,
            0.0,
            TextFormat {
                font_id: FontId::monospace(14.0),
                color,
                ..Default::default()
            },
        );
    };
    if !rust || text.len() > 200_000 {
        append(&mut job, text, plain);
        return job;
    }
    let mut index = 0;
    while index < text.len() {
        let rest = &text[index..];
        let first = rest.chars().next().unwrap();
        let (len, color) = if rest.starts_with("//") {
            (
                rest.find('\n').unwrap_or(rest.len()),
                Color32::from_rgb(123, 148, 134),
            )
        } else if rest.starts_with("/*") {
            let bytes = rest.as_bytes();
            let mut depth = 1;
            let mut end = 2;
            while end < bytes.len() && depth > 0 {
                if bytes[end..].starts_with(b"/*") {
                    depth += 1;
                    end += 2;
                } else if bytes[end..].starts_with(b"*/") {
                    depth -= 1;
                    end += 2;
                } else {
                    end += 1;
                }
            }
            (end, Color32::from_rgb(123, 148, 134))
        } else if first == '"' {
            let mut escaped = false;
            let mut end = first.len_utf8();
            for ch in rest[end..].chars() {
                end += ch.len_utf8();
                if ch == '"' && !escaped {
                    break;
                }
                escaped = ch == '\\' && !escaped;
            }
            (end, Color32::from_rgb(214, 193, 141))
        } else if first.is_alphabetic() || first == '_' {
            let end = rest
                .char_indices()
                .find(|(_, ch)| !ch.is_alphanumeric() && *ch != '_')
                .map(|(i, _)| i)
                .unwrap_or(rest.len());
            let word = &rest[..end];
            let keyword = matches!(
                word,
                "fn" | "pub"
                    | "use"
                    | "mod"
                    | "let"
                    | "mut"
                    | "const"
                    | "static"
                    | "struct"
                    | "enum"
                    | "impl"
                    | "trait"
                    | "type"
                    | "where"
                    | "for"
                    | "in"
                    | "if"
                    | "else"
                    | "match"
                    | "while"
                    | "loop"
                    | "return"
                    | "break"
                    | "continue"
                    | "async"
                    | "await"
                    | "move"
                    | "ref"
                    | "self"
                    | "Self"
                    | "super"
                    | "crate"
                    | "true"
                    | "false"
                    | "unsafe"
                    | "dyn"
                    | "as"
            );
            (
                end,
                if keyword {
                    Color32::from_rgb(168, 157, 238)
                } else {
                    plain
                },
            )
        } else if first.is_ascii_digit() {
            let end = rest
                .char_indices()
                .find(|(_, ch)| !ch.is_ascii_alphanumeric() && *ch != '_' && *ch != '.')
                .map(|(i, _)| i)
                .unwrap_or(rest.len());
            (end, Color32::from_rgb(136, 200, 212))
        } else {
            (first.len_utf8(), plain)
        };
        append(&mut job, &text[index..index + len], color);
        index += len;
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coloring_preserves_unicode_comments_and_every_source_byte() {
        let source = "fn main() { /* nested /* café */ α */ let text = \"你好 \\\"\"; // note\n}\n";
        let output = highlight(source, true);
        assert_eq!(output.text, source);
        let mut end = 0;
        for section in &output.sections {
            assert_eq!(section.byte_range.start, end);
            end = section.byte_range.end;
            assert!(source.is_char_boundary(end));
        }
        assert_eq!(end, source.len());
        assert!(output.sections.len() > 5);
    }
}
