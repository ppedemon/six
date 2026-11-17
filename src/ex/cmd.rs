use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{escaped, tag, take_while1},
    character::complete::{anychar, char, none_of, space0},
    combinator::{complete, fail, opt, recognize},
    multi::{many1, separated_list1},
    sequence::delimited,
};

use crate::ex::{ExError, ExRange, range::parse_exrange};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltIn {
    Quit {
        bang: bool,
    },
    Exit {
        bang: bool,
    },
    Write {
        bang: bool,
        append: bool,
        quit: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExCmd<'a> {
    ExRange(ExRange),
    BuiltIn {
        range: ExRange,
        builtin: BuiltIn,
        args: &'a str,
    },
    Shell {
        range: ExRange,
        raw_cmd: &'a str,
    },
}

fn opt_bang(input: &str) -> IResult<&str, bool> {
    opt(char('!')).map(|bang| bang.is_some()).parse(input)
}

fn opt_append(input: &str) -> IResult<&str, bool> {
    opt((space0, tag(">>"), space0))
        .map(|o| o.is_some())
        .parse(input)
}

fn parse_builtin_cmd(input: &str) -> IResult<&str, BuiltIn> {
    let (rest, cmd) = take_while1(|c: char| c.is_ascii_alphabetic()).parse(input)?;
    match cmd {
        "q" | "quit" => opt_bang.map(|bang| BuiltIn::Quit { bang }).parse(rest),
        "x" | "xit" => opt_bang.map(|bang| BuiltIn::Exit { bang }).parse(rest),
        "wq" => (opt_bang, opt_append)
            .map(|(bang, append)| BuiltIn::Write {
                bang,
                append,
                quit: true,
            })
            .parse(rest),
        "w" | "write" => (opt_bang, opt_append)
            .map(|(bang, append)| BuiltIn::Write {
                bang,
                append,
                quit: false,
            })
            .parse(rest),
        _ => fail().parse(rest),
    }
}

fn parse_builtin(input: &str) -> IResult<&str, BuiltIn> {
    delimited(space0, parse_builtin_cmd, space0).parse(input)
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

pub fn parse_cmd_line(input: &str) -> Result<Vec<ExCmd<'_>>, ExError> {
    let separator = complete((space0, char('|'), space0));

    let (rest, cmds) = separated_list1(
        separator,
        alt((
            parse_shell,
            parse_internal,
            parse_exrange.map(ExCmd::ExRange),
        )),
    )
    .parse(input)
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
    use crate::ex::range::{Address, BaseAddress, Delimiter, Modifier};

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
                builtin: BuiltIn::Write {
                    bang: false,
                    append: false,
                    quit: false,
                },
                args: ""
            }
        );

        let cmd_line = "q !   ";
        let result = parse_cmd_line(cmd_line);
        assert_eq!(
            result.unwrap()[0],
            ExCmd::BuiltIn {
                range: ExRange::Implicit,
                builtin: BuiltIn::Quit { bang: false },
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
                builtin: BuiltIn::Write {
                    bang: true,
                    append: true,
                    quit: false,
                },
                args: r"test\|file.txt",
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
                builtin: BuiltIn::Write {
                    bang: false,
                    append: false,
                    quit: false,
                },
                args: ""
            }
        );
        assert_eq!(
            result[1],
            ExCmd::BuiltIn {
                range: ExRange::Implicit,
                builtin: BuiltIn::Quit { bang: false },
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
                builtin: BuiltIn::Write {
                    bang: false,
                    append: false,
                    quit: false,
                },
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
                builtin: BuiltIn::Write {
                    bang: false,
                    append: false,
                    quit: false,
                },
                args: "file.txt"
            }
        );

        assert_eq!(
            result[1],
            ExCmd::BuiltIn {
                range: ExRange::Single {
                    address: address(BaseAddress::Current)
                },
                builtin: BuiltIn::Write {
                    bang: true,
                    append: false,
                    quit: false,
                },
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
    fn test_parse_range() {
        let cmd_line = "1;10 20";
        let result = parse_cmd_line(cmd_line).unwrap();
        assert_eq!(
            result[0],
            ExCmd::ExRange(ExRange::explicit(
                address(BaseAddress::Line(1)),
                address_mod(BaseAddress::Line(10), vec![Modifier::Plus(20)]),
                Delimiter::Semi
            ))
        )
    }
}
