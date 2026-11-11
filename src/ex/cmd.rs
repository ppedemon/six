use std::{collections::HashMap, sync::LazyLock};

use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{escaped, take_while1},
    character::{
        char,
        complete::{anychar, digit1, none_of, space0},
    },
    combinator::{complete, opt, recognize},
    multi::{many1, separated_list1},
    sequence::delimited,
};

use crate::ex::{ExError, ExRange, range::parse_exrange};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltIn<'a> {
    Quit(&'a str),
    Write(&'a str),
    GotoLine(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExCmd<'a> {
    BuiltIn {
        range: ExRange,
        builtin: BuiltIn<'a>,
        args: &'a str,
    },
    Shell {
        range: ExRange,
        raw_cmd: &'a str,
    },
}

static BUILTIN_CMDS: LazyLock<HashMap<&str, BuiltIn<'static>>> = LazyLock::new(|| {
    let mut builtins = HashMap::new();
    builtins.insert("q!", BuiltIn::Quit("q!"));
    builtins.insert("q", BuiltIn::Quit("q"));
    builtins.insert("wq!", BuiltIn::Quit("wq!"));
    builtins.insert("wq", BuiltIn::Quit("wq"));
    builtins.insert("x!", BuiltIn::Quit("x!"));
    builtins.insert("x", BuiltIn::Quit("x"));
    builtins.insert("w!", BuiltIn::Write("w!"));
    builtins.insert("w", BuiltIn::Write("w"));
    builtins
});

fn parse_builtin(input: &str) -> IResult<&str, BuiltIn<'_>> {
    delimited(
        space0,
        take_while1(|c: char| c.is_alphanumeric() || c == '!'),
        space0,
    )
    .map_opt(|s| BUILTIN_CMDS.get(s).copied())
    .parse(input)
}

fn parse_shell(input: &str) -> IResult<&str, ExCmd<'_>> {
    (parse_exrange, char('!'), space0, recognize(many1(anychar)))
        .map(|(range, _, _, raw_cmd)| ExCmd::Shell {
            range,
            raw_cmd: raw_cmd.trim(),
        })
        .parse(input)
}

fn parse_internal(input: &str) -> IResult<&str, ExCmd<'_>> {
    (
        parse_exrange,
        parse_builtin,
        opt(escaped(none_of(r"\|"), '\\', anychar)),
    )
        .map(|(range, builtin, args)| ExCmd::BuiltIn {
            range,
            builtin,
            args: args.unwrap_or("").trim(),
        })
        .parse(input)
}

fn parse_num(input: &str) -> IResult<&str, ExCmd<'_>> {
    (
        space0::<&str, _>,
        digit1.map_res(|s: &str| s.parse::<usize>()),
        space0,
    )
        .map(|(_, d, _)| ExCmd::BuiltIn {
            range: ExRange::Implicit,
            builtin: BuiltIn::GotoLine(d),
            args: "",
        })
        .parse(input)
}

pub fn parse_cmd_line(input: &str) -> Result<Vec<ExCmd<'_>>, ExError> {
    let separator = complete((space0, char('|'), space0));

    let (rest, cmds) = separated_list1(separator, alt((parse_shell, parse_internal)))
        .parse(input)
        .or_else(|_| parse_num.map(|cmd| vec![cmd]).parse(input))
        .map_err(|_| ExError::ParseError {
            cmd: input.to_string(),
        })?;

    if rest.trim().len() > 0 {
        return Err(ExError::ParseError {
            cmd: input.to_string(),
        });
    }

    Ok(cmds)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ex::{
        BuiltIn::GotoLine,
        range::{Address, BaseAddress, Delimiter, Modifier},
    };

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
    fn test_parse_internal() {
        let cmd_line = "w";
        let result = parse_cmd_line(cmd_line);
        assert_eq!(
            result.unwrap()[0],
            ExCmd::BuiltIn {
                range: ExRange::Implicit,
                builtin: BuiltIn::Write("w"),
                args: ""
            }
        );

        let cmd_line = "q !   ";
        let result = parse_cmd_line(cmd_line);
        assert_eq!(
            result.unwrap()[0],
            ExCmd::BuiltIn {
                range: ExRange::Implicit,
                builtin: BuiltIn::Quit("q"),
                args: "!"
            }
        );

        let cmd_line = r".;+3w! >> test\|file.txt";
        let result = parse_cmd_line(cmd_line);
        assert_eq!(
            result.unwrap()[0],
            ExCmd::BuiltIn {
                range: ExRange::Explicit {
                    start: address(BaseAddress::Current),
                    end: address_mod(BaseAddress::Current, vec![Modifier::Plus(3)]),
                    delimiter: Delimiter::Semi,
                },
                builtin: BuiltIn::Write("w!"),
                args: r">> test\|file.txt",
            }
        );
    }

    #[test]
    fn test_parse_shell() {
        let cmd_line = "! ls -l";
        let result = parse_cmd_line(cmd_line);
        assert_eq!(
            result.unwrap()[0],
            ExCmd::Shell {
                range: ExRange::Implicit,
                raw_cmd: "ls -l",
            }
        );

        let cmd_line = ". ! ls -l";
        let result = parse_cmd_line(cmd_line);
        assert_eq!(
            result.unwrap()[0],
            ExCmd::Shell {
                range: ExRange::Single {
                    address: address(BaseAddress::Current)
                },
                raw_cmd: "ls -l",
            }
        );
    }

    #[test]
    fn test_trivial_piping() {
        let cmd_line = "   w | q ";
        let result = parse_cmd_line(cmd_line).unwrap();
        assert_eq!(
            result[0],
            ExCmd::BuiltIn {
                range: ExRange::Implicit,
                builtin: BuiltIn::Write("w"),
                args: ""
            }
        );
        assert_eq!(
            result[1],
            ExCmd::BuiltIn {
                range: ExRange::Implicit,
                builtin: BuiltIn::Quit("q"),
                args: ""
            }
        );
    }

    #[test]
    fn test_piping() {
        let cmd_line = "w file.txt | !grep foo | grep baz";
        let result = parse_cmd_line(cmd_line).unwrap();
        assert_eq!(
            result[0],
            ExCmd::BuiltIn {
                range: ExRange::Implicit,
                builtin: BuiltIn::Write("w"),
                args: "file.txt"
            }
        );

        assert_eq!(
            result[1],
            ExCmd::Shell {
                range: ExRange::Implicit,
                raw_cmd: "grep foo | grep baz"
            }
        );
    }

    #[test]
    fn test_filtered_piping() {
        let cmd_line = r"w file.txt | .w! out.txt | .!ls -l | grep file\.txt | wc -l   ";
        let result = parse_cmd_line(cmd_line).unwrap();
        assert_eq!(
            result[0],
            ExCmd::BuiltIn {
                range: ExRange::Implicit,
                builtin: BuiltIn::Write("w"),
                args: "file.txt"
            }
        );

        assert_eq!(
            result[1],
            ExCmd::BuiltIn {
                range: ExRange::Single {
                    address: address(BaseAddress::Current)
                },
                builtin: BuiltIn::Write("w!"),
                args: "out.txt"
            }
        );

        assert_eq!(
            result[2],
            ExCmd::Shell {
                range: ExRange::Single {
                    address: address(BaseAddress::Current)
                },
                raw_cmd: r"ls -l | grep file\.txt | wc -l",
            }
        );
    }

    #[test]
    fn test_parse_numline() {
        let cmd_line = "10";
        let result = parse_cmd_line(cmd_line).unwrap();
        assert_eq!(
            result[0],
            ExCmd::BuiltIn {
                range: ExRange::Implicit,
                builtin: GotoLine(10),
                args: ""
            }
        )
    }
}
