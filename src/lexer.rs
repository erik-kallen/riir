use crate::htab::HashTable;
use std::ffi::CString;

#[repr(C)]
pub struct LexerContext<'a> {
    tokens: Vec<Vec<&'a str>>,
}

fn lex<'a>(source: &'a str, defines: &'a HashTable) -> Vec<Vec<&'a str>> {
    source
        .lines()
        .map(|line| {
            // Ignore everything after comment
            let line = match line.find("#") {
                Some(i) => line.split_at(i).0,
                None => line,
            };

            let line_tokens: Vec<_> = line
                .split(&[' ', '\t', ','][..])
                .filter(|token| token.len() > 0)
                .map(|token| {
                    let token = match defines.0.get(&CString::new(token).unwrap()) {
                        Some(item) => match item.opaque_value_str() {
                            Some(value) => value,
                            None => token,
                        },
                        None => token,
                    };

                    token
                })
                .collect();

            line_tokens
        })
        .collect()
}

impl<'a> LexerContext<'a> {
    fn empty() -> LexerContext<'a> {
        LexerContext { tokens: vec![] }
    }

    fn lex_into_self(self: &mut LexerContext<'a>, source: &'a str, defines: &'a HashTable) {
        self.tokens = lex(source, defines);
    }

    pub fn lex(source: &'a str, defines: &'a HashTable) -> LexerContext<'a> {
        let mut lexer = LexerContext::empty();
        lexer.lex_into_self(source, defines);
        lexer
    }

    pub fn tokens(self: &LexerContext<'a>) -> &Vec<Vec<&str>> {
        &self.tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::htab::{HashTable, Item};

    fn run_test(source: &str, expected_tokens: &[&[&str]]) {
        run_test_with_defines(source, &[], expected_tokens);
    }

    fn run_test_with_defines(source: &str, defines: &[(&str, &str)], expected_tokens: &[&[&str]]) {
        let mut defines_htab = HashTable::default();
        for define in defines {
            defines_htab.0.insert(
                CString::new(define.0).unwrap(),
                Item::opaque(CString::new(define.1).unwrap()),
            );
        }

        let actual: Vec<_> = lex(source, &defines_htab);

        assert_eq!(actual, expected_tokens);
    }

    #[test]
    fn can_lex_single_line_without_newline() {
        run_test("mov eax, 1", &[&["mov", "eax", "1"]])
    }

    #[test]
    fn can_lex_single_line_with_newline() {
        run_test("mov eax, 1\n", &[&["mov", "eax", "1"]])
    }

    #[test]
    fn can_lex_multiple_lines() {
        run_test(
            "mov eax, 1\ninc ebx\npushf\nadd eax, 2",
            &[
                &["mov", "eax", "1"],
                &["inc", "ebx"],
                &["pushf"],
                &["add", "eax", "2"],
            ],
        );
    }

    #[test]
    fn multiple_spaces_and_tabs_are_ignored() {
        run_test("  mov  \t  eax  ,\t  1", &[&["mov", "eax", "1"]]);
    }

    #[test]
    fn everything_after_comment_start_until_end_of_line_is_ignored() {
        run_test(
            "mov eax, 1 #  this is some comment\n  # some other comment\n#Line comment\ndec eax#Comment\ninc eax",
            &[&["mov", "eax", "1"], &[], &[], &["dec", "eax"], &["inc", "eax"]],
        );
    }

    #[test]
    fn can_substitute_defines() {
        run_test_with_defines(
            "mov target, source",
            &[("target", "eax"), ("source", "21")],
            &[&["mov", "eax", "21"]],
        );
    }

    #[test]
    fn can_lex_lines_with_windows_line_endings() {
        run_test("inc eax\r\ninc ebx", &[&["inc", "eax"], &["inc", "ebx"]]);
    }
}
