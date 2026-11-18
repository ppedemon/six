use core::fmt;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::escaped,
    character::complete::{anychar, char, digit1, none_of, space0},
    combinator::{opt, value},
    multi::many0,
    sequence::{delimited, preceded},
};
use regex::{Regex, RegexBuilder};
use std::{fmt::Write, unreachable, write};

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

impl fmt::Display for SearchPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Backward(regex) => write!(f, "?{}?", regex),
            Self::Forward(regex) => write!(f, "/{}/", regex),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseAddress {
    Zero,
    Current,
    Last,
    Line(usize),
    Mark(char),
    Pattern(SearchPattern),
}

impl fmt::Display for BaseAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Zero => write!(f, "0"),
            Self::Current => write!(f, "."),
            Self::Last => write!(f, "$"),
            Self::Line(line) => write!(f, "{line}"),
            Self::Mark(c) => write!(f, "'{c}"),
            Self::Pattern(p) => fmt::Display::fmt(p, f),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modifier {
    Plus(usize),
    Minus(usize),
}

impl fmt::Display for Modifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plus(n) => write!(f, "+{n}"),
            Self::Minus(n) => write!(f, "-{n}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub base: BaseAddress,
    pub modifiers: Vec<Modifier>,
}

impl Address {
    pub fn current() -> Self {
        Self {
            base: BaseAddress::Current,
            modifiers: Vec::new(),
        }
    }

    pub fn current_plus(offset: usize) -> Self {
        Self {
            base: BaseAddress::Current,
            modifiers: vec![Modifier::Plus(offset)],
        }
    }

    pub fn first() -> Self {
        Self {
            base: BaseAddress::Line(1),
            modifiers: Vec::new(),
        }
    }

    pub fn last() -> Self {
        Self {
            base: BaseAddress::Last,
            modifiers: Vec::new(),
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Display::fmt(&self.base, f)?;
        self.modifiers
            .iter()
            .map(|m| fmt::Display::fmt(m, f))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delimiter {
    Comma,
    Semi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExRange {
    Implicit,
    List {
        head: Address,
        tail: Vec<(Delimiter, Address)>,
    },
}

impl ExRange {
    pub fn all() -> Self {
        Self::List {
            head: Address::first(),
            tail: vec![(Delimiter::Comma, Address::last())],
        }
    }

    pub fn single(address: Address) -> Self {
        Self::List {
            head: address,
            tail: Vec::new(),
        }
    }

    pub fn pair(start: Address, delimiter: Delimiter, end: Address) -> Self {
        Self::List {
            head: start,
            tail: vec![(delimiter, end)],
        }
    }

    pub fn coerce_implicit_to(self, scope: ExRange) -> ExRange {
        match self {
            ExRange::Implicit => scope,
            _ => self,
        }
    }
}

impl fmt::Display for ExRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Implicit => std::fmt::Result::Ok(()),
            Self::List { head, tail } => {
                write!(f, "{head}")?;
                for (delimiter, address) in tail {
                    let delim_c = match delimiter {
                        Delimiter::Comma => ',',
                        Delimiter::Semi => ';',
                    };
                    write!(f, "{delim_c}{address}")?;
                }
                std::fmt::Result::Ok(())
            }
        }
    }
}

fn parse_modifier(input: &str) -> IResult<&str, Modifier> {
    alt((
        (
            space0,
            digit1.map(|s: &str| s.parse::<usize>().unwrap_or(usize::MAX)),
        )
            .map(|(_, n)| Modifier::Plus(n)),
        (
            space0,
            alt((char('+'), char('-'))),
            opt(digit1.map(|s: &str| s.parse::<usize>().unwrap_or(usize::MAX))),
        )
            .map(|(_, sign, n)| match sign {
                '+' => Modifier::Plus(n.unwrap_or(1)),
                '-' => Modifier::Minus(n.unwrap_or(1)),
                _ => unreachable!("Invalid modifier sign"),
            }),
    ))
    .parse(input)
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
        .map_res(|s| RegexBuilder::new(&s).multi_line(true).build())
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
            .map(|s: &str| s.parse::<usize>().unwrap_or(usize::MAX))
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

#[derive(Clone)]
enum XAddress {
    All,
    Address(Address),
}

fn parse_xaddress(input: &str) -> IResult<&str, XAddress> {
    alt((
        value(
            XAddress::All,
            delimited(space0::<&str, nom::error::Error<_>>, char('%'), space0),
        ),
        parse_address.map(XAddress::Address),
    ))
    .parse(input)
}

pub fn parse_exrange(input: &str) -> IResult<&str, ExRange> {
    let result = (parse_xaddress, many0((parse_delimiter, parse_xaddress)))
        .map(|(h, t)| {
            let mut tail = Vec::with_capacity(t.len() * 2);
            let head = match h {
                XAddress::All => {
                    tail.push((Delimiter::Comma, Address::last()));
                    Address::first()
                }
                XAddress::Address(a) => a,
            };
            for (delim, xaddress) in t {
                match xaddress {
                    XAddress::All => {
                        tail.push((delim, Address::first()));
                        tail.push((Delimiter::Comma, Address::last()));
                    }
                    XAddress::Address(a) => tail.push((delim, a)),
                }
            }
            ExRange::List { head, tail }
        })
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
        ExRange::List {
            head: address,
            tail: Vec::new(),
        }
    }

    fn explicit(start: Address, end: Address, delimiter: Delimiter) -> ExRange {
        ExRange::List {
            head: start,
            tail: vec![(delimiter, end)],
        }
    }

    fn all() -> ExRange {
        explicit(Address::first(), Address::last(), Delimiter::Comma)
    }

    fn assert_parse(input: &str, expected_spec: ExRange, expected_rem: &str) {
        let res = parse_exrange(input);
        assert!(res.is_ok(), "Failed to parse '{}': {:?}", input, res.err());
        let (rem, spec) = res.unwrap();
        assert_eq!(spec, expected_spec, "Unexpected result for '{}'", input);
        assert_eq!(rem, expected_rem, "Unexpected remainder for '{}'", input);
    }

    #[test]
    fn test_all() {
        assert_parse("%", all(), "");
        assert_parse("  %  ", all(), "");
        assert_parse("%d", all(), "d");
        assert_parse("  %  d", all(), "d");
    }

    #[test]
    fn test_implicit() {
        assert_parse("", ExRange::Implicit, "");
        assert_parse("d", ExRange::Implicit, "d");
        assert_parse("   d", ExRange::Implicit, "d");
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
            "  .  +1  ;  $  -2  d  ",
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

    #[test]
    fn test_ex_manual_ranges() {
        assert_parse(
            "+++",
            singleton(address_mod(
                BaseAddress::Current,
                vec![Modifier::Plus(1), Modifier::Plus(1), Modifier::Plus(1)],
            )),
            "",
        );

        assert_parse(
            "/^aaa.*/-",
            singleton(address_mod(
                BaseAddress::Pattern(SearchPattern::Forward(Regex::new("^aaa.*").unwrap())),
                vec![Modifier::Minus(1)],
            )),
            "",
        );

        assert_parse(
            "-2",
            singleton(address_mod(BaseAddress::Current, vec![Modifier::Minus(2)])),
            "",
        );

        assert_parse(
            "3 ---- 2",
            singleton(address_mod(
                BaseAddress::Line(3),
                vec![
                    Modifier::Minus(1),
                    Modifier::Minus(1),
                    Modifier::Minus(1),
                    Modifier::Minus(1),
                    Modifier::Plus(2),
                ],
            )),
            "",
        );

        assert_parse(
            "1 2 3",
            singleton(address_mod(
                BaseAddress::Line(1),
                vec![Modifier::Plus(2), Modifier::Plus(3)],
            )),
            "",
        );
    }
}
