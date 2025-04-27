#![allow(dead_code)]
use winnow::{
    Result,
    ascii::{line_ending, space0, till_line_ending},
    combinator::{alt, preceded, repeat, separated, seq},
    prelude::*,
    stream::AsChar,
    token::take_while,
};

fn shell<'a>(input: &mut &'a str) -> Result<&'a str> {
    alt(("bash", "fish", "zsh", "tcsh")).parse_next(input)
}

fn list_sep<'a>(input: &mut &'a str) -> Result<(&'a str, &'a str)> {
    (",", space0).parse_next(input)
}

fn shells_decorator<'a>(input: &mut &'a str) -> Result<Vec<&'a str>> {
    ("@shells(", separated(1..=4, shell, list_sep), ")")
        .parse_next(input)
        .map(|(_, parsed_shells, _)| parsed_shells)
}

fn test_name<'a>(input: &mut &'a str) -> Result<&'a str> {
    take_while(1.., (AsChar::is_alphanum, '_')).parse_next(input)
}

fn test_header<'a>(input: &mut &'a str) -> Result<&'a str> {
    preceded("@test ", test_name).parse_next(input)
}

fn line<'a>(input: &mut &'a str) -> Result<(&'a str, &'a str)> {
    seq!(till_line_ending, line_ending).parse_next(input)
}

fn test_body<'a>(input: &mut &'a str) -> Result<Vec<(&'a str, &'a str)>> {
    repeat(1.., line).parse_next(input)
}

#[cfg(test)]
mod tests {
    use indoc::formatdoc;

    use super::*;

    #[test]
    fn parses_shell_decorator_no_spaces() {
        let mut input = "@shells(bash,fish)";
        let shells = shells_decorator(&mut input).unwrap();
        assert_eq!(shells, vec!["bash", "fish"]);
    }

    #[test]
    fn parses_shell_decorator_with_spaces() {
        let mut input = "@shells(bash, fish)";
        let shells = shells_decorator(&mut input).unwrap();
        assert_eq!(shells, vec!["bash", "fish"]);
    }

    #[test]
    fn parses_test_name() {
        let mut names = ["foo", "foo_bar", "foo_bar1", "1foo", "_foo"];
        for name in names.iter_mut() {
            let name_copy = name.to_string();
            let parsed = test_name(name).unwrap();
            assert_eq!(parsed, name_copy.as_str());
        }
    }

    #[test]
    fn parses_test_body() {
        let input = formatdoc! {"
            foo
            bar
            baz
        "};
        let parsed = test_body(&mut input.as_str()).unwrap();
        assert_eq!(parsed, vec![("foo", "\n"), ("bar", "\n"), ("baz", "\n")]);
    }
}
