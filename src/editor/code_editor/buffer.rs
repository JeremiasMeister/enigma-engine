use egui::TextBuffer;

/// egui's built-in character limit does not bound UTF-8 or CRLF bytes on disk.
pub struct BoundedText<'a> {
    pub text: &'a mut String,
    pub capacity: usize,
    pub crlf: bool,
    pub limited: &'a mut bool,
}

impl TextBuffer for BoundedText<'_> {
    fn is_mutable(&self) -> bool {
        true
    }
    fn as_str(&self) -> &str {
        self.text
    }
    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        let text = text.replace("\r\n", "\n");
        let used = self.text.len()
            + if self.crlf {
                self.text.bytes().filter(|b| *b == b'\n').count()
            } else {
                0
            };
        let mut remaining = self.capacity.saturating_sub(used);
        let mut end = 0;
        for ch in text.chars() {
            let cost = ch.len_utf8() + usize::from(self.crlf && ch == '\n');
            if cost > remaining || ch == '\0' {
                break;
            }
            remaining -= cost;
            end += ch.len_utf8();
        }
        *self.limited |= end < text.len();
        self.text.insert_text(&text[..end], char_index)
    }
    fn delete_char_range(&mut self, range: std::ops::Range<usize>) {
        self.text.delete_char_range(range);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_and_crlf_input_stay_within_the_serialized_byte_limit() {
        let mut text = "a".into();
        let mut limited = false;
        let mut buffer = BoundedText {
            text: &mut text,
            capacity: 8,
            crlf: true,
            limited: &mut limited,
        };
        assert_eq!(buffer.insert_text("世\r\n🙂", 1), 2);
        assert_eq!(buffer.as_str(), "a世\n");
        assert!(limited);
        assert_eq!(text.as_str().replace('\n', "\r\n").len(), 6);
    }
}
