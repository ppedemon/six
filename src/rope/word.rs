use ropey::Rope;

pub fn is_sub_word_char(c: char) -> bool {
    c == '_' || (!c.is_whitespace() && !c.is_ascii_punctuation() && !c.is_control())
}

fn is_punct_char(c: char) -> bool {
    (c.is_ascii_punctuation() || c.is_control()) && !c.is_whitespace()
}

pub fn next_big_word(rope: &Rope, char_idx: usize) -> usize {
    if rope.len_chars() == 0 {
        return 0;
    }

    let max_idx = rope.len_chars().saturating_sub(1);
    let mut char_idx = char_idx.min(max_idx);
    let mut c = rope.char(char_idx);

    while char_idx < max_idx && !c.is_whitespace() {
        char_idx += 1;
        c = rope.char(char_idx);
    }

    if char_idx < max_idx && c == '\n' && rope.char(char_idx + 1) == '\n' {
        return char_idx + 1;
    }

    while char_idx < max_idx && c.is_whitespace() {
        char_idx += 1;
        c = rope.char(char_idx);
    }

    if char_idx == max_idx && rope.char(char_idx) == '\n' {
        char_idx += 1;
    }

    char_idx
}

pub fn next_sub_word(rope: &Rope, char_idx: usize) -> usize {
    if rope.len_chars() == 0 {
        return 0;
    }

    let max_idx = rope.len_chars().saturating_sub(1);
    let mut char_idx = char_idx.min(max_idx);
    let mut c = rope.char(char_idx);

    if is_sub_word_char(c) {
        while char_idx < max_idx && is_sub_word_char(c) {
            char_idx += 1;
            c = rope.char(char_idx);
        }
    } else if is_punct_char(c) {
        while char_idx < max_idx && is_punct_char(c) {
            char_idx += 1;
            c = rope.char(char_idx);
        }
    }

    if char_idx < max_idx && c == '\n' && rope.char(char_idx + 1) == '\n' {
        return char_idx + 1;
    }

    while char_idx < max_idx && c.is_whitespace() {
        char_idx += 1;
        c = rope.char(char_idx);
    }

    if char_idx == max_idx && rope.char(char_idx) == '\n' {
        char_idx += 1;
    }

    char_idx
}

pub fn prev_big_word(rope: &Rope, char_idx: usize) -> usize {
    let max_idx = rope.len_chars().saturating_sub(1);
    if max_idx == 0 {
        return 0;
    }

    if char_idx > max_idx {
        return max_idx;
    }

    let mut char_idx = char_idx;
    let mut c = rope.char(char_idx.saturating_sub(1));

    if char_idx > 1 && c == '\n' && rope.char(char_idx - 2) == '\n' {
        return char_idx - 1;
    }

    while char_idx != 0 && c.is_whitespace() {
        char_idx -= 1;
        c = rope.char(char_idx.saturating_sub(1));
    }

    while char_idx != 0 && !c.is_whitespace() {
        char_idx -= 1;
        c = rope.char(char_idx.saturating_sub(1));
    }

    char_idx
}

pub fn prev_sub_word(rope: &Rope, char_idx: usize) -> usize {
    let max_idx = rope.len_chars().saturating_sub(1);
    if max_idx == 0 {
        return 0;
    }

    if char_idx > max_idx {
        return max_idx;
    }

    let mut char_idx = char_idx;
    let mut c = rope.char(char_idx.saturating_sub(1));

    if char_idx > 1 && c == '\n' && rope.char(char_idx - 2) == '\n' {
        return char_idx - 1;
    }

    while char_idx != 0 && c.is_whitespace() {
        char_idx -= 1;
        c = rope.char(char_idx.saturating_sub(1));
    }

    if is_sub_word_char(c) {
        while char_idx != 0 && is_sub_word_char(c) {
            char_idx -= 1;
            c = rope.char(char_idx.saturating_sub(1));
        }
    } else if is_punct_char(c) {
        while char_idx != 0 && is_punct_char(c) {
            char_idx -= 1;
            c = rope.char(char_idx.saturating_sub(1));
        }
    }

    char_idx
}

pub fn end_big_word(rope: &Rope, char_idx: usize) -> usize {
    let max_idx = rope.len_chars().saturating_sub(1);
    if max_idx == 0 {
        return 0;
    }

    let mut char_idx = char_idx.clamp(0, max_idx);
    let mut c = rope.char(char_idx.saturating_add(1).min(max_idx));

    while char_idx < max_idx && c.is_whitespace() {
        char_idx += 1;
        c = rope.char(char_idx.saturating_add(1).min(max_idx));
    }

    while char_idx < max_idx && !c.is_whitespace() {
        char_idx += 1;
        c = rope.char(char_idx.saturating_add(1).min(max_idx));
    }

    if char_idx == max_idx && rope.char(char_idx) == '\n' {
        char_idx += 1;
    }

    char_idx
}

pub fn end_sub_word(rope: &Rope, char_idx: usize) -> usize {
    let max_idx = rope.len_chars().saturating_sub(1);
    if max_idx == 0 {
        return 0;
    }

    let mut char_idx = char_idx.clamp(0, max_idx);
    let mut c = rope.char(char_idx.saturating_add(1).min(max_idx));

    while char_idx < max_idx && c.is_whitespace() {
        char_idx += 1;
        c = rope.char(char_idx.saturating_add(1).min(max_idx));
    }

    if is_sub_word_char(c) {
        while char_idx < max_idx && is_sub_word_char(c) {
            char_idx += 1;
            c = rope.char(char_idx.saturating_add(1).min(max_idx));
        }
    } else if is_punct_char(c) {
        while char_idx < max_idx && is_punct_char(c) {
            char_idx += 1;
            c = rope.char(char_idx.saturating_add(1).min(max_idx));
        }
    }

    if char_idx == max_idx && rope.char(char_idx) == '\n' {
        char_idx += 1;
    }

    char_idx
}

#[cfg(test)]
mod word_ws_tests {
    use super::*;
    use ropey::Rope;

    macro_rules! check_jump {
        ($func:ident, $text:expr, $char_idx: expr, Expected => $line:expr) => {
            let rope = Rope::from_str($text);
            let res = $func(&rope, $char_idx);
            assert_eq!(res, $line, "Failed on text: {:?}", $text);
        };
    }

    #[test]
    fn test_next_word_empty() {
        check_jump!(next_big_word, "", 0, Expected => 0);
        check_jump!(next_big_word, "", 20, Expected => 0);
    }

    #[test]
    fn test_next_word_spaces() {
        check_jump!(next_big_word, "    ", 0, Expected => 3);
    }

    #[test]
    fn test_next_word_standard_jump() {
        // Cursor on 'm' of 'mut' -> should jump to 'f' of 'foo'
        check_jump!(next_big_word, "mut foo", 0, Expected => 4);

        // Cursor on 'u' of 'mut' -> should still jump to 'f' of 'foo'
        check_jump!(next_big_word, "mut foo", 1, Expected => 4);

        // Cursor on 'f' of 'foo' -> should jump to last index
        check_jump!(next_big_word, "mut foo", 4, Expected => 6);
    }

    #[test]
    fn test_next_word_multiple_spaces() {
        // Should cross multiple spaces seamlessly
        check_jump!(next_big_word, "let    value", 0, Expected => 7);
    }

    #[test]
    fn test_next_word_crosses_newlines() {
        // Should drop down to the next line if a word ends near a newline
        let text = "first\nsecond";
        check_jump!(next_big_word, text, 0, Expected => 6);

        // Following vi behaviour, next word jumps between empty lines
        let blank_line_text = "first\n\n\n  second";
        check_jump!(next_big_word, blank_line_text, 5, Expected => 6);
    }

    #[test]
    fn test_next_word_boundaries() {
        // Last word in the file has no next word -> returns None
        check_jump!(next_big_word, "tail", 0, Expected => 3);
        check_jump!(next_big_word, "tail   ", 0, Expected => 6);

        // Out of bounds inputs
        check_jump!(next_big_word, "hello", 5, Expected => 4);
    }

    #[test]
    fn test_next_word_punct_and_emojis() {
        check_jump!(next_sub_word, "   -A-  !", 4, Expected => 5);
        check_jump!(next_big_word, "a_b a", 0, Expected => 4);
        check_jump!(next_sub_word, "a_b-- a", 0, Expected => 3);
        check_jump!(next_big_word, "a🧑‍🧑‍🧒‍🧒b a", 0, Expected => 10);
        check_jump!(next_sub_word, "a🧑‍🧑‍🧒‍🧒b a", 0, Expected => 10);
    }

    #[test]
    fn test_prev_word_empty() {
        check_jump!(prev_big_word, "", 0, Expected => 0);
        check_jump!(prev_big_word, "", 20, Expected => 0);
    }

    #[test]
    fn test_prev_word_spaces() {
        check_jump!(prev_big_word, "    ", 3, Expected => 0);
    }

    #[test]
    fn test_prev_word_standard_jump() {
        // Cursor on 'f' of 'foo' -> should jump back to 'm' of 'mut'
        check_jump!(prev_big_word, "mut foo", 4, Expected => 0);

        // Cursor in the middle of 'foo' -> should jump back to 'f'
        check_jump!(prev_big_word, "mut foo", 5, Expected => 4);
    }

    #[test]
    fn test_prev_word_multiple_spaces() {
        // Should cross multiple spaces backwards seamlessly
        check_jump!(prev_big_word, "first let    value", 13, Expected => 6);

        // Should go to the beginning of the current word
        check_jump!(prev_big_word, "first let    value", 7, Expected => 6);
    }

    #[test]
    fn test_prev_word_lands_at_start_of_file() {
        // Your specific unwrap_or edge-case: jumping to the very first word of a file
        check_jump!(prev_big_word, "first_word  ", 11, Expected => 0);
        check_jump!(prev_big_word, "first_word", 5, Expected => 0);
    }

    #[test]
    fn test_prev_word_crosses_newlines_backward() {
        // Should move up a line if spaces/newlines are encountered backwards
        let text = "first\nsecond";
        check_jump!(prev_big_word, text, 6, Expected => 0);

        let multi_line = "first\n\n\n  second";
        check_jump!(prev_big_word, multi_line, 10, Expected => 0);

        let multi_line_1 = "first\n\n\n  second third";
        check_jump!(prev_big_word, multi_line_1, 17, Expected => 10);
    }

    #[test]
    fn test_prev_word_punct_and_emojis() {
        check_jump!(prev_big_word, "a_b a", 4, Expected => 0);
        check_jump!(prev_sub_word, "a_b-- a", 6, Expected => 3);
        check_jump!(prev_big_word, "a🧑‍🧑‍🧒‍🧒b a", 10, Expected => 0);
        check_jump!(prev_sub_word, "a🧑‍🧑‍🧒‍🧒b a", 10, Expected => 0);
    }

    #[test]
    fn test_ctrl_chars() {
        check_jump!(next_sub_word, "a\n\u{1}a", 2, Expected => 3);
        check_jump!(next_sub_word, "a\n\u{1}a", 0, Expected => 2);
        check_jump!(prev_sub_word, "a\n\u{1}a", 3, Expected => 2);
        check_jump!(prev_sub_word, "a\n\u{1}a", 2, Expected => 0);
    }
}
