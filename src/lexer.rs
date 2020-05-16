use std::collections::HashMap;

#[repr(C)]
pub struct LexerContext<'a> {
    tokens: Vec<Vec<&'a str>>,
}

impl<'a> LexerContext<'a> {
    pub fn lex(source: &'a str, defines: &'a HashMap<String, String>) -> LexerContext<'a> {
        let tokens = source
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
                        let token = match defines.get(token) {
                            Some(value) => value,
                            None => token,
                        };
    
                        token
                    })
                    .collect();
    
                line_tokens
            })
            .collect();

        LexerContext { tokens }
    }

    pub fn tokens(self: &LexerContext<'a>) -> &[Vec<&str>] {
        &self.tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_test(source: &str, expected_tokens: &[&[&str]]) {
        run_test_with_defines(source, &[], expected_tokens);
    }

    fn run_test_with_defines(source: &str, defines: &[(&str, &str)], expected_tokens: &[&[&str]]) {
        let mut defines_htab = HashMap::<String, String>::default();
        for define in defines {
            defines_htab.insert(define.0.to_owned(), define.1.to_owned());
        }

        let actual = LexerContext::lex(source, &defines_htab);

        assert_eq!(actual.tokens, expected_tokens);
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
