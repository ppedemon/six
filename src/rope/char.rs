use ropey::Rope;

pub fn find_char_forward(rope: &Rope, c: char, reps: usize, char_idx: usize) -> usize {
    scan_char_forward(rope, c, reps, char_idx, true)
}

pub fn till_char_forward(rope: &Rope, c: char, reps: usize, char_idx: usize) -> usize {
    scan_char_forward(rope, c, reps, char_idx, false)
}

fn scan_char_forward(rope: &Rope, c: char, reps: usize, char_idx: usize, inclusive: bool) -> usize {
    let max_idx = rope.len_chars().saturating_sub(1);
    let mut i = char_idx.min(max_idx);

    let line_idx = rope.char_to_line(char_idx);
    let line_start = rope.line_to_char(line_idx);
    let line_end = line_start.saturating_add(rope.line(line_idx).len_chars());

    let mut n = reps;
    while i < line_end.saturating_sub(1) {
        if rope.char(i.saturating_add(1)) == c {
            n -= 1;
            if n == 0 {
                return if inclusive { i.saturating_add(1) } else { i };
            }
        }
        i += 1;
    }

    char_idx
}

pub fn find_char_backward(rope: &Rope, c: char, reps: usize, char_idx: usize) -> usize {
    scan_char_backward(rope, c, reps, char_idx, true)
}

pub fn till_char_backward(rope: &Rope, c: char, reps: usize, char_idx: usize) -> usize {
    scan_char_backward(rope, c, reps, char_idx, false)
}

fn scan_char_backward(
    rope: &Rope,
    c: char,
    reps: usize,
    char_idx: usize,
    inclusive: bool,
) -> usize {
    let max_idx = rope.len_chars().saturating_sub(1);
    let mut i = char_idx.min(max_idx);

    let line_idx = rope.char_to_line(char_idx);
    let line_start = rope.line_to_char(line_idx);

    let mut n = reps;
    while i > line_start {
        if rope.char(i.saturating_sub(1)) == c {
            n -= 1;
            if n == 0 {
                return if inclusive { i.saturating_sub(1) } else { i };
            }
        }
        i -= 1;
    }

    char_idx
}
