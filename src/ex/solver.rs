use std::ops::Range;

use ropey::Rope;

use crate::{
    ex::{
        ExError, ExRange,
        range::{Address, BaseAddress, Delimiter, Modifier, SearchPattern},
    },
    rope,
};

fn solve_pattern(pattern: SearchPattern, rope: &Rope, curr_line: usize) -> Result<usize, ExError> {
    let search_result = match pattern {
        SearchPattern::Backward(regex) => {
            rope::find_backward(rope, &regex, curr_line.saturating_sub(1), None)
        }
        SearchPattern::Forward(regex) => {
            rope::find_forward(rope, &regex, curr_line.saturating_add(1), None)
        }
    };

    match search_result {
        None => Err(ExError::PatternNotFound),
        Some((start_char_idx, _)) => Ok(rope.char_to_line(start_char_idx)),
    }
}

// Preconditions:
//  - relative_to is a 1-based line number
//  - The line number in BaseAdress:Line is 1-based
// Postconditions:
//  - The returned solved line number is 1-based
fn solve_base_address(
    base_address: BaseAddress,
    rope: &Rope,
    relative_to: usize,
) -> Result<usize, ExError> {
    match base_address {
        BaseAddress::Zero => Ok(0),

        // relative_to is 1-based, we can return it directly
        BaseAddress::Current => Ok(relative_to),

        // rope.len_lines() points to the last line when counting 1-based
        BaseAddress::Last => Ok(rope.len_lines()),

        // BaseAddress::Line is already 1-based
        BaseAddress::Line(line_idx) => {
            if line_idx > rope.len_lines() {
                Err(ExError::InvalidRange)
            } else {
                Ok(line_idx)
            }
        }

        // solve_pattern works with 0-based line numbers
        BaseAddress::Pattern(pattern) => {
            let start_line = relative_to.saturating_sub(1);
            let line_idx = solve_pattern(pattern, rope, start_line)?;
            Ok(line_idx + 1)
        }

        BaseAddress::Mark(m) => Err(ExError::UnsupportedAddress {
            address: format!("'{m}"),
        }),
    }
}

// Preconditions:
//  - relative_to is a 1-based line number
// Postconditions:
//  - The returned solved line number is 1-based
fn solve_address(address: Address, rope: &Rope, relative_to: usize) -> Result<usize, ExError> {
    let mut base_address = solve_base_address(address.base, rope, relative_to)?;
    for modifier in address.modifiers {
        match modifier {
            Modifier::Minus(amount) => {
                if base_address < amount {
                    return Err(ExError::InvalidRange);
                }
                base_address -= amount;
            }
            Modifier::Plus(amount) => {
                base_address = base_address.saturating_add(amount);
                if base_address > rope.len_lines() {
                    return Err(ExError::InvalidRange);
                }
            }
        }
    }
    Ok(base_address)
}

// Preconditions:
//  - curr_line is a 0-based line number
// Postconditions:
//  - The returned range is a normal 0-based [inclusive, exclusive) range
pub fn solve_exrange(range: ExRange, rope: &Rope, curr_line: usize) -> Result<Range<usize>, ExError> {
    match range {
        ExRange::All => Ok(0..rope.len_lines()),
        ExRange::Implicit => Ok(curr_line..curr_line + 1),
        ExRange::Single { address } => {
            let base_1_line = solve_address(address, rope, curr_line + 1)?;
            Ok(base_1_line.saturating_sub(1)..base_1_line)
        }
        ExRange::Explicit {
            start,
            end,
            delimiter,
        } => {
            let base_1_start = solve_address(start, rope, curr_line + 1)?;
            let relative_to = match delimiter {
                Delimiter::Comma => curr_line + 1,
                Delimiter::Semi => base_1_start,
            };
            let base_1_end = solve_address(end, rope, relative_to)?;
            Ok(base_1_start.saturating_sub(1)..base_1_end)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use regex::Regex;
    use ropey::Rope;

    fn address(base: BaseAddress) -> Address {
        Address {
            base,
            modifiers: vec![],
        }
    }

    fn address_mod(base: BaseAddress, modifiers: Vec<Modifier>) -> Address {
        Address { base, modifiers }
    }

    #[test]
    fn test_solve_base_address_basic() {
        let rope = Rope::from("line1\nline2\nline3");
        let relative_to = 2; // 1-based (Line 2)

        // BaseAddress::Zero -> returns 1-based 0
        assert_eq!(
            solve_base_address(BaseAddress::Zero, &rope, relative_to).unwrap(),
            0
        );

        // BaseAddress::Current -> returns relative_to
        assert_eq!(
            solve_base_address(BaseAddress::Current, &rope, relative_to).unwrap(),
            2,
        );

        // BaseAddress::Last -> returns total lines (1-based)
        assert_eq!(
            solve_base_address(BaseAddress::Last, &rope, relative_to).unwrap(),
            3,
        );

        // BaseAddress::Line(n) -> returns n directly
        assert!(matches!(
            solve_base_address(BaseAddress::Line(5), &rope, relative_to),
            Err(ExError::InvalidRange),
        ));

        // BaseAddress::Mark -> Unsupported
        assert!(matches!(
            solve_base_address(BaseAddress::Mark('a'), &rope, relative_to),
            Err(ExError::UnsupportedAddress { .. }),
        ));
    }

    #[test]
    fn test_solve_base_address_patterns() {
        let rope = Rope::from("apple\nbanana\ncherry\nbanana banana");

        // Forward search from line 1 (idx 0) looking for "cherry"
        let p_forward = BaseAddress::Pattern(SearchPattern::Forward(Regex::new("cherry").unwrap()));
        assert_eq!(solve_base_address(p_forward, &rope, 1).unwrap(), 3); // Line 3

        // Backward search from line 4 (idx 3) looking for "banana"
        let p_backward =
            BaseAddress::Pattern(SearchPattern::Backward(Regex::new("banana").unwrap()));
        assert_eq!(solve_base_address(p_backward, &rope, 4).unwrap(), 2); // Line 1

        // Forward pattern not found error propagation (we don't check in the current line)
        let p_missing = BaseAddress::Pattern(SearchPattern::Forward(Regex::new("cherry").unwrap()));
        assert!(matches!(
            solve_base_address(p_missing, &rope, 3),
            Err(ExError::PatternNotFound),
        ));

        // Backward pattern not found error propagation (we don't check in the current line)
        let p_missing =
            BaseAddress::Pattern(SearchPattern::Backward(Regex::new("cherry").unwrap()));
        assert!(matches!(
            solve_base_address(p_missing, &rope, 3),
            Err(ExError::PatternNotFound),
        ));
    }

    #[test]
    fn test_solve_address_modifiers() {
        let rope = Rope::from("l1\nl2\nl3\nl4\nl5");

        // Target: Current + 2 - 1 => 2 + 2 - 1 = 3
        let addr = address_mod(
            BaseAddress::Current,
            vec![Modifier::Plus(2), Modifier::Minus(1)],
        );
        assert_eq!(solve_address(addr, &rope, 2).unwrap(), 3);

        // Underflow test: saturating_sub protection via 0 address
        let addr_underflow = address_mod(BaseAddress::Zero, vec![Modifier::Minus(5)]);
        assert!(matches!(
            solve_address(addr_underflow, &rope, 2),
            Err(ExError::InvalidRange)
        ));

        // Overflow test: yields InvalidRange
        let addr_overflow = address_mod(BaseAddress::Line(usize::MAX), vec![Modifier::Plus(10)]);
        assert!(matches!(
            solve_address(addr_overflow, &rope, 2),
            Err(ExError::InvalidRange)
        ));
    }

    #[test]
    fn test_solve_scope_all_and_implicit() {
        let rope = Rope::from("l1\nl2\nl3");

        // Scope::All -> 0..len_lines (0..3)
        assert_eq!(solve_exrange(ExRange::All, &rope, 1).unwrap(), 0..3);

        // Scope::Implicit -> curr_line..curr_line + 1 (1..2)
        assert_eq!(solve_exrange(ExRange::Implicit, &rope, 1).unwrap(), 1..2);
    }

    #[test]
    fn test_solve_scope_single_address() {
        let rope = Rope::from("l1\nl2\nl3");

        // Target single address line 2 (transforms to 0-based index 1..2)
        let scope = ExRange::Single {
            address: address(BaseAddress::Line(2)),
        };
        assert_eq!(solve_exrange(scope, &rope, 0).unwrap(), 1..2);

        // Target single address line 0 (saturating_sub converts 1-based 0 to 0..0)
        let scope_zero = ExRange::Single {
            address: address(BaseAddress::Zero),
        };
        assert_eq!(solve_exrange(scope_zero, &rope, 0).unwrap(), 0..0);
    }

    #[test]
    fn test_solve_scope_explicit_comma() {
        let rope = Rope::from("l1\nl2\nl3\nl4\nl5");

        // Range: 2, . + 1  while cursor is on 0-based index 1 (Line 2)
        // Comma evaluates both relative to original cursor (relative_to = 2)
        let scope = ExRange::Explicit {
            start: address(BaseAddress::Line(2)),
            end: address_mod(BaseAddress::Current, vec![Modifier::Plus(1)]),
            delimiter: Delimiter::Comma,
        };

        // base_1_start = 2 (idx 1).
        // base_1_end = relative_to(2) + 1 = 3 (idx 3).
        // Expected 0-based range: 1..3
        assert_eq!(solve_exrange(scope, &rope, 1).unwrap(), 1..3);
    }

    #[test]
    fn test_solve_scope_explicit_semicolon() {
        let rope = Rope::from("apple\nbanana\ncherry\ndate\nelderberry");

        // Range: /cherry/ ; +2  while cursor is on 0-based index 0 ("apple")
        // Semicolon updates relative_to baseline for the end address calculation
        let scope = ExRange::Explicit {
            start: address(BaseAddress::Pattern(SearchPattern::Forward(
                Regex::new("cherry").unwrap(),
            ))),
            end: address_mod(BaseAddress::Current, vec![Modifier::Plus(2)]), // In our code, Current resolves to the baseline passed
            delimiter: Delimiter::Semi,
        };

        // start finds "cherry" at line 3 (idx 2).
        // Semicolon sets relative_to = 3.
        // end calculates relative to 3: 3 + 2 = 5 (idx 5).
        // Expected 0-based range: 2..5
        assert_eq!(solve_exrange(scope, &rope, 0).unwrap(), 2..5);
    }

    #[test]
    fn test_solve_scope_explicit_semicolon_with_zero() {
        let rope = Rope::from("l1\nl2\nl3");

        // Range: 0 ; +1
        // Baseline updates to 0, end address adds 1 to it.
        let scope = ExRange::Explicit {
            start: address(BaseAddress::Zero),
            end: address_mod(BaseAddress::Current, vec![Modifier::Plus(1)]),
            delimiter: Delimiter::Semi,
        };

        // start = 0
        // Semicolon sets relative_to = 0.
        // end evaluates Current(0) + 1 = 1.
        // Expected 0-based range: 0..1
        assert_eq!(solve_exrange(scope, &rope, 2).unwrap(), 0..1);
    }
}
