use std::{fmt::Write, write};

use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{escaped, take_while1},
    character::complete::{anychar, char, none_of, one_of},
    combinator::{opt, value, verify},
    multi::separated_list1,
};
use regex::Regex;

use crate::{
    components::{Direction, Offset, OffsetSide, OffsetSign, PatSearch},
    ex::ExError,
};

fn parse_side(input: &str) -> IResult<&str, OffsetSide> {
    alt((
        value(OffsetSide::End, char('e')),
        value(OffsetSide::Start, one_of("sb")),
    ))
    .parse(input)
}

fn parse_sign(input: &str) -> IResult<&str, OffsetSign> {
    alt((
        value(OffsetSign::Plus, char('+')),
        value(OffsetSign::Minus, char('-')),
    ))
    .parse(input)
}

fn parse_stride(input: &str) -> IResult<&str, usize> {
    take_while1(|c: char| c.is_ascii_digit())
        .map_res(|s: &str| s.parse::<usize>())
        .parse(input)
}

fn parse_offset(input: &str) -> IResult<&str, Offset> {
    alt((
        (parse_side, parse_sign, parse_stride).map(|(side, sign, stride)| Offset::Char {
            side,
            sign,
            stride,
        }),
        (parse_side, parse_stride).map(|(side, stride)| Offset::Char {
            side,
            sign: OffsetSign::Plus,
            stride,
        }),
        (parse_sign, parse_stride).map(|(sign, stride)| Offset::Line { sign, stride }),
        parse_stride.map(|stride| Offset::Line {
            sign: OffsetSign::Plus,
            stride,
        }),
        parse_sign.map(|sign| Offset::Line { sign, stride: 1 }),
        parse_side.map(|side| match side {
            OffsetSide::Start => Offset::Empty,
            OffsetSide::End => Offset::Inclusive,
        }),
    ))
    .parse(input)
}

fn parse_pat(c: char) -> impl Fn(&str) -> IResult<&str, Regex> {
    let mut escape_seq = String::with_capacity(2);
    write!(&mut escape_seq, r"\{}", c).unwrap();
    move |input| {
        verify(
            escaped(none_of(escape_seq.as_str()), '\\', anychar),
            |s: &str| !s.is_empty(),
        )
        .map_res(Regex::new)
        .parse(input)
    }
}

pub fn parse_one_search(
    c: char,
    direction: Direction,
) -> impl Fn(&str) -> IResult<&str, PatSearch> {
    move |input| {
        alt((
            (char(c), char(c), opt(parse_offset)).map(|(_, _, opt_offset)| PatSearch {
                direction,
                pat: None,
                offset: opt_offset.unwrap_or(Offset::Empty),
            }),
            (char(c), parse_pat(c), char(c), parse_offset).map(|(_, pat, _, offset)| PatSearch {
                direction,
                pat: Some(pat),
                offset,
            }),
            (char(c), parse_pat(c), opt(char(c))).map(|(_, pat, _)| PatSearch {
                direction,
                pat: Some(pat),
                offset: Offset::Empty,
            }),
            value(
                PatSearch {
                    direction,
                    pat: None,
                    offset: Offset::Empty,
                },
                char(c),
            ),
        ))
        .parse(input)
    }
}

pub fn parse_search(input: &str) -> Result<Vec<PatSearch>, ExError> {
    let (rest, pats) = separated_list1(
        char(';'),
        alt((
            parse_one_search('/', Direction::Forward),
            parse_one_search('?', Direction::Backward),
        )),
    )
    .parse(input)
    .map_err(|_| ExError::InvalidSearchPattern {
        pat: input.to_owned(),
    })?;

    if rest.trim().len() > 0 {
        return Err(ExError::InvalidSearchPattern {
            pat: input.to_owned(),
        });
    }

    Ok(pats)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_side_sign_and_stride() {
        assert!(matches!(parse_side("e!"), Ok(("!", OffsetSide::End))));
        assert!(matches!(parse_side("s!"), Ok(("!", OffsetSide::Start))));
        assert!(matches!(parse_side("b!"), Ok(("!", OffsetSide::Start))));

        assert!(matches!(parse_sign("+x"), Ok(("x", OffsetSign::Plus))));
        assert!(matches!(parse_sign("-x"), Ok(("x", OffsetSign::Minus))));
        assert!(matches!(parse_stride("42x"), Ok(("x", 42))));
    }

    #[test]
    fn parse_offset_forms() {
        assert!(matches!(
            parse_offset("e-3!"),
            Ok((
                "!",
                Offset::Char {
                    side: OffsetSide::End,
                    sign: OffsetSign::Minus,
                    stride: 3,
                }
            ))
        ));
        assert!(matches!(
            parse_offset("s4!"),
            Ok((
                "!",
                Offset::Char {
                    side: OffsetSide::Start,
                    sign: OffsetSign::Plus,
                    stride: 4,
                }
            ))
        ));
        assert!(matches!(
            parse_offset("-2!"),
            Ok((
                "!",
                Offset::Line {
                    sign: OffsetSign::Minus,
                    stride: 2
                }
            ))
        ));
        assert!(matches!(
            parse_offset("5!"),
            Ok((
                "!",
                Offset::Line {
                    sign: OffsetSign::Plus,
                    stride: 5
                }
            ))
        ));
        assert!(matches!(
            parse_offset("+!"),
            Ok((
                "!",
                Offset::Line {
                    sign: OffsetSign::Plus,
                    stride: 1
                }
            ))
        ));
        assert!(matches!(
            parse_offset("-!"),
            Ok((
                "!",
                Offset::Line {
                    sign: OffsetSign::Minus,
                    stride: 1
                }
            ))
        ));
        assert!(matches!(parse_offset("s!"), Ok(("!", Offset::Empty))));
        assert!(matches!(parse_offset("e!"), Ok(("!", Offset::Inclusive))));
    }

    #[test]
    fn parse_search_patterns_and_offsets() {
        let searches = parse_search(r"/foo\/bar/+2;?baz?-3").unwrap();
        assert_eq!(searches.len(), 2);

        assert!(matches!(searches[0].direction, Direction::Forward));
        assert_eq!(searches[0].pat.as_ref().unwrap().as_str(), r"foo\/bar");
        assert!(matches!(
            searches[0].offset,
            Offset::Line {
                sign: OffsetSign::Plus,
                stride: 2
            }
        ));

        assert!(matches!(searches[1].direction, Direction::Backward));
        assert_eq!(searches[1].pat.as_ref().unwrap().as_str(), "baz");
        assert!(matches!(
            searches[1].offset,
            Offset::Line {
                sign: OffsetSign::Minus,
                stride: 3
            }
        ));
    }

    #[test]
    fn parse_search_handles_unterminated_and_repeated_patterns() {
        let unterminated = parse_search("/foo").unwrap();
        assert_eq!(unterminated[0].pat.as_ref().unwrap().as_str(), "foo");
        assert!(matches!(unterminated[0].offset, Offset::Empty));

        let terminated = parse_search("/foo/").unwrap();
        assert_eq!(terminated[0].pat.as_ref().unwrap().as_str(), "foo");
        assert!(matches!(terminated[0].offset, Offset::Empty));

        let repeated = parse_search("//+4").unwrap();
        assert!(repeated[0].pat.is_none());
        assert!(matches!(
            repeated[0].offset,
            Offset::Line {
                sign: OffsetSign::Plus,
                stride: 4
            }
        ));
    }

    #[test]
    fn parse_search_rejects_invalid_regex() {
        assert!(matches!(
            parse_search("/[ /"),
            Err(ExError::InvalidSearchPattern { .. })
        ));
    }
}
