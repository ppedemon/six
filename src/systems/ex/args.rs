use nom::{
    Parser,
    bytes::complete::escaped,
    character::complete::{anychar, none_of, space1},
    multi::separated_list0,
};

use crate::misc::path::norm_filename;
use crate::{components::BufferName, ex::ExError};

pub fn validate_no_args(args: &str) -> Result<(), ExError> {
    validate_args(args, |args| {
        if args.is_empty() {
            Ok(())
        } else {
            Err(ExError::invalid_args("trailing args"))
        }
    })
}

pub fn validate_opt_filename(args: &str) -> Result<Option<BufferName>, ExError> {
    validate_args(args, |args| match args.len() {
        0 => Ok(None),
        1 => {
            let file_path = norm_filename(args[0]);
            let path_str = file_path.as_os_str();
            if path_str.is_empty() {
                return Err(ExError::invalid_args("Invalid file name"));
            }
            Ok(Some(BufferName::new(args[0], file_path)))
        }
        _ => Err(ExError::invalid_args("Only one filename allowed")),
    })
}

pub fn validate_args<T>(
    args: &str,
    validator: impl Fn(Vec<&str>) -> Result<T, ExError>,
) -> Result<T, ExError> {
    let arg_vec = space_tokenize(args)?;
    validator(arg_vec)
}

fn space_tokenize(s: &str) -> Result<Vec<&str>, ExError> {
    let s = s.trim();
    let tok_parser = escaped(none_of(r"\ "), '\\', anychar::<_, nom::error::Error<_>>);
    let mut toks = separated_list0(space1, tok_parser);
    let result = toks.parse(s);
    match result {
        Ok((_, tokens)) => Ok(tokens
            .into_iter()
            .filter(|s| s.trim().len() > 0)
            .collect::<Vec<_>>()),
        Err(_) => Err(ExError::ParseError {
            cmd: format!("Invalid args: {}", s),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_space_tokenize() {
        let args = r" tok1   tok2  tok3 ";
        let toks = space_tokenize(args).unwrap();
        assert_eq!(toks, vec!["tok1", "tok2", "tok3"]);

        let args = r" token\ with\ many\ spaces\ \ end   tok2  tok3 ";
        let toks = space_tokenize(args).unwrap();
        assert_eq!(
            toks,
            vec![r"token\ with\ many\ spaces\ \ end", "tok2", "tok3"]
        );
    }
}
