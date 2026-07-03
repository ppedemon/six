use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::escaped,
    character::complete::{anychar, char, digit1, none_of, space0},
    combinator::{opt, value},
    multi::many0,
    sequence::{delimited, preceded},
};
use regex::Regex;
use std::fmt::Write;

#[derive(Debug, Clone)]
pub enum SearchPattern {
    Forward(Regex),
    Backward(Regex),
}

impl PartialEq for SearchPattern {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SearchPattern::Forward(a), SearchPattern::Forward(b)) => a.as_str() == b.as_str(),
            (SearchPattern::Backward(a), SearchPattern::Backward(b)) => a.as_str() == b.as_str(),
            _ => false,
        }
    }
}

impl Eq for SearchPattern {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseAddress {
    Zero,
    Current,
    Last,
    Line(usize),
    Mark(char),
    Pattern(SearchPattern),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modifier {
    Plus(usize),
    Minus(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub base: BaseAddress,
    pub modifiers: Vec<Modifier>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delimiter {
    Comma,
    Semi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExRange {
    All,
    Implicit,
    Single {
        address: Address,
    },
    Explicit {
        start: Address,
        end: Address,
        delimiter: Delimiter,
    },
}

impl ExRange {
    pub fn coerce_implicit_to(self, scope: ExRange) -> ExRange {
        match self {
            ExRange::Implicit => scope,
            _ => self,
        }
    }
}

fn parse_modifier(input: &str) -> IResult<&str, Modifier> {
    let (input, (_, sign, _, amount)) = (
        space0,
        alt((char('+'), char('-'))),
        space0,
        opt(digit1.map_res(|s: &str| s.parse::<usize>())),
    )
        .parse(input)?;

    let modifier = match sign {
        '+' => Ok(Modifier::Plus(amount.unwrap_or(1))),
        '-' => Ok(Modifier::Minus(amount.unwrap_or(1))),
        _ => unreachable!("Invalid modifier sign"),
    }?;

    Ok((input, modifier))
}

fn parse_delimited_regex<'a>(delim: char) -> impl Fn(&str) -> IResult<&str, Regex> {
    let mut escape_seq = String::with_capacity(2);
    write!(&mut escape_seq, r"\{}", delim).unwrap();

    let mut delim_str = String::with_capacity(1);
    write!(&mut delim_str, "{}", delim).unwrap();

    move |input| {
        let normal = none_of(escape_seq.as_str());
        delimited(
            char::<&str, _>(delim),
            escaped(normal, '\\', anychar),
            char(delim),
        )
        .map(|s| s.replace(escape_seq.as_str(), delim_str.as_str()))
        .map_res(|s| Regex::new(&s))
        .parse(input)
    }
}

fn parse_search_pattern(input: &str) -> IResult<&str, SearchPattern> {
    alt((
        parse_delimited_regex('/').map(SearchPattern::Forward),
        parse_delimited_regex('?').map(SearchPattern::Backward),
    ))
    .parse(input)
}

fn parse_base_address(input: &str) -> IResult<&str, BaseAddress> {
    alt((
        value(BaseAddress::Zero, char('0')),
        value(BaseAddress::Current, char('.')),
        value(BaseAddress::Last, char('$')),
        digit1
            .map_res(|s: &str| s.parse::<usize>())
            .map(BaseAddress::Line),
        preceded(char('\''), anychar).map(BaseAddress::Mark),
        parse_search_pattern.map(BaseAddress::Pattern),
    ))
    .parse(input)
}

fn parse_address(input: &str) -> IResult<&str, Address> {
    let (input, (_, base_opt, _, modifiers, _)) = (
        space0,
        opt(parse_base_address),
        space0,
        many0(parse_modifier),
        space0,
    )
        .parse(input)?;

    let base = base_opt.unwrap_or(BaseAddress::Current);
    Ok((input, Address { base, modifiers }))
}

fn parse_delimiter(input: &str) -> IResult<&str, Delimiter> {
    delimited(
        space0,
        alt((
            value(Delimiter::Comma, char(',')),
            value(Delimiter::Semi, char(';')),
        )),
        space0,
    )
    .parse(input)
}

pub fn parse_exrange(input: &str) -> IResult<&str, ExRange> {
    let result = alt((
        delimited(space0, value(ExRange::All, char('%')), space0),
        (parse_address, parse_delimiter, parse_address).map(|(start, delimiter, end)| {
            ExRange::Explicit {
                start,
                end,
                delimiter,
            }
        }),
        parse_address.map(|addr| ExRange::Single { address: addr }),
    ))
    .parse(input);

    match result {
        Ok((rest, range_spec)) => {
            if rest.len() == input.trim_start().len() {
                Ok((rest, ExRange::Implicit))
            } else {
                Ok((rest, range_spec))
            }
        }
        _ => result,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn assert_parse(input: &str, expected_spec: ExRange, expected_rem: &str) {
        let res = parse_exrange(input);
        assert!(res.is_ok(), "Failed to parse '{}': {:?}", input, res.err());
        let (rem, spec) = res.unwrap();
        assert_eq!(spec, expected_spec, "Unexpected result for '{}'", input);
        assert_eq!(rem, expected_rem, "Unexpected remainder for '{}'", input);
    }

    #[test]
    fn test_all() {
        assert_parse("%", ExRange::All, "");
        assert_parse("  %  ", ExRange::All, "");
        assert_parse("%d", ExRange::All, "d");
        assert_parse("  %  d", ExRange::All, "d");
    }

    #[test]
    fn test_implicit() {
        assert_parse("", ExRange::Implicit, "");
        assert_parse("d", ExRange::Implicit, "d");
        assert_parse("   d", ExRange::Implicit, "d");
    }

    fn address(base: BaseAddress) -> Address {
        Address {
            base,
            modifiers: vec![],
        }
    }

    fn address_mod(base: BaseAddress, modifiers: Vec<Modifier>) -> Address {
        Address { base, modifiers }
    }

    fn singleton(address: Address) -> ExRange {
        ExRange::Single { address }
    }

    fn explicit(start: Address, end: Address, delimiter: Delimiter) -> ExRange {
        ExRange::Explicit {
            start,
            end,
            delimiter,
        }
    }

    #[test]
    fn test_base_address_variants() {
        assert_parse(".", singleton(address(BaseAddress::Current)), "");
        assert_parse("$", singleton(address(BaseAddress::Last)), "");
        assert_parse("0", singleton(address(BaseAddress::Zero)), "");
        assert_parse("42", singleton(address(BaseAddress::Line(42))), "");
        assert_parse("'a", singleton(address(BaseAddress::Mark('a'))), "");

        let re = Regex::new(r"a\((b+)?\)c").unwrap();
        assert_parse(
            r"?a\((b+)\?\)c?",
            singleton(address(BaseAddress::Pattern(SearchPattern::Backward(re)))),
            "",
        );

        let re = Regex::new(r"a/b*/c").unwrap();
        assert_parse(
            r"/a\/b*\/c/",
            singleton(address(BaseAddress::Pattern(SearchPattern::Forward(re)))),
            "",
        );
    }

    #[test]
    fn test_modifiers_and_chaining() {
        assert_parse(
            "+5",
            singleton(address_mod(BaseAddress::Current, vec![Modifier::Plus(5)])),
            "",
        );

        assert_parse(
            ".-",
            singleton(address_mod(BaseAddress::Current, vec![Modifier::Minus(1)])),
            "",
        );

        let modifiers = vec![Modifier::Plus(2), Modifier::Minus(3), Modifier::Plus(10)];
        assert_parse(
            "$+2-3+10d",
            singleton(address_mod(BaseAddress::Last, modifiers)),
            "d",
        );
    }

    #[test]
    fn test_explicit_full_scopes() {
        assert_parse(
            "5,10",
            explicit(
                address(BaseAddress::Line(5)),
                address(BaseAddress::Line(10)),
                Delimiter::Comma,
            ),
            "",
        );

        assert_parse(
            "+;$-2",
            explicit(
                address_mod(BaseAddress::Current, vec![Modifier::Plus(1)]),
                address_mod(BaseAddress::Last, vec![Modifier::Minus(2)]),
                Delimiter::Semi,
            ),
            "",
        );

        assert_parse("0", singleton(address(BaseAddress::Zero)), "");
    }

    #[test]
    fn test_arbitrary_whitespace() {
        assert_parse(
            "  .  +  1  ;  $  -  2  d  ",
            explicit(
                address_mod(BaseAddress::Current, vec![Modifier::Plus(1)]),
                address_mod(BaseAddress::Last, vec![Modifier::Minus(2)]),
                Delimiter::Semi,
            ),
            "d  ",
        );

        assert_parse(
            "  42  ,  'b  ",
            explicit(
                address(BaseAddress::Line(42)),
                address(BaseAddress::Mark('b')),
                Delimiter::Comma,
            ),
            "",
        );

        let start_re = Regex::new("ab?").unwrap();
        let end_re = Regex::new(r"http://github\.com/").unwrap();
        assert_parse(
            r"  ?ab\??  ,  /http:\/\/github\.com\//  ",
            explicit(
                address(BaseAddress::Pattern(SearchPattern::Backward(start_re))),
                address(BaseAddress::Pattern(SearchPattern::Forward(end_re))),
                Delimiter::Comma,
            ),
            "",
        );
    }
}
