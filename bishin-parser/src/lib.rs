use std::path::{Path, PathBuf};

use winnow::Parser;

pub use crate::parser::Test;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to parse test file '{0}': {1}")]
    Parse(PathBuf, String),
    #[error("failed to read test file: '{0}'")]
    Io(PathBuf, #[source] std::io::Error),
}

/// Returns the tests parsed from a test file.
pub fn parse_test_file(path: impl AsRef<Path>) -> Result<Vec<Test>, Error> {
    let path = path.as_ref().to_path_buf();
    let contents = std::fs::read_to_string(&path).map_err(|err| Error::Io(path.clone(), err))?;
    let tests = parser::test_file
        .parse(&contents)
        .map_err(|err| Error::Parse(path.clone(), err.inner().to_string()))?
        .into_iter()
        .map(|borrowed| borrowed.to_test())
        .collect::<Vec<_>>();
    Ok(tests)
}

#[cfg(test)]
mod tests {
    use indoc::formatdoc;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn public_parser_fn() {
        let input = formatdoc! {"

    
           @test test1 {{
               foo
           }}

           @test test2 {{
               bar
           }}
           @test test3 {{
               baz
           }}
        "};

        let tempdir = TempDir::new().unwrap();
        let path = tempdir.path().join("test.b");
        std::fs::write(&path, input).unwrap();
        let tests = parse_test_file(path).unwrap();
        assert_eq!(tests.len(), 3);
        assert_eq!(tests[0].name, "test1".to_string());
        assert_eq!(tests[1].name, "test2".to_string());
        assert_eq!(tests[2].name, "test3".to_string());
    }
}

mod parser {

    use winnow::{
        Result,
        ascii::{line_ending, multispace0, space0, till_line_ending},
        combinator::{alt, preceded, repeat, separated, seq, terminated},
        prelude::*,
        stream::AsChar,
        token::take_while,
    };

    /// An unprocessed, parsed test that borrows from the input.
    #[derive(Debug)]
    pub(crate) struct BorrowedTest<'a> {
        name: &'a str,
        // (line, line_ending)
        body: Vec<(&'a str, &'a str)>,
    }

    impl BorrowedTest<'_> {
        pub(crate) fn to_test(&self) -> Test {
            let name = self.name.to_string();
            let mut body = String::new();
            self.body.iter().for_each(|(line, ending)| {
                body.push_str(line);
                body.push_str(ending);
            });
            Test { name, body }
        }
    }

    /// A parsed test.
    #[derive(Debug, Clone)]
    pub struct Test {
        pub name: String,
        pub body: String,
    }

    #[allow(dead_code)]
    fn shell<'a>(input: &mut &'a str) -> Result<&'a str> {
        alt(("bash", "fish", "zsh", "tcsh")).parse_next(input)
    }

    #[allow(dead_code)]
    fn list_sep<'a>(input: &mut &'a str) -> Result<(&'a str, &'a str)> {
        (",", space0).parse_next(input)
    }

    #[allow(dead_code)]
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
        seq!(till_line_ending, line_ending)
            .verify(|&(l, _): &(&str, &str)| !l.starts_with('}'))
            .parse_next(input)
    }

    fn test_body<'a>(input: &mut &'a str) -> Result<Vec<(&'a str, &'a str)>> {
        repeat(1.., line).parse_next(input)
    }

    fn test<'a>(input: &mut &'a str) -> Result<BorrowedTest<'a>> {
        let name = test_header.parse_next(input)?;
        let begin = (" {", line_ending);
        let body_and_end = terminated(test_body, ("}", line_ending));
        let body = preceded(begin, body_and_end).parse_next(input)?;
        Ok(BorrowedTest { name, body })
    }

    pub(crate) fn test_file<'a>(input: &mut &'a str) -> Result<Vec<BorrowedTest<'a>>> {
        preceded(multispace0, repeat(0.., terminated(test, multispace0))).parse_next(input)
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

        #[test]
        fn parses_test() {
            let input = formatdoc! {"
           @test test_name {{
               foo
               bar
           }}
        "};
            let parsed = test(&mut input.as_str()).unwrap();
            assert_eq!(parsed.name, "test_name");
            assert_eq!(parsed.body, vec![("    foo", "\n"), ("    bar", "\n")]);
        }

        #[test]
        fn parses_test_file() {
            let input = formatdoc! {"

        
               @test test1 {{
                   foo
               }}

               @test test2 {{
                   bar
               }}
               @test test3 {{
                   baz
               }}
            "};
            let parsed = test_file(&mut input.as_str()).unwrap();
            assert_eq!(parsed.len(), 3);
        }
    }
}
