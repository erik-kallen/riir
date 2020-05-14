#[cfg(test)]
mod tests {
    use crate::ffi;
    use std::ffi::{CStr, CString};

    fn run_test(source: &str, expected_tokens: &[&[&str]]) {
        run_test_with_defines(source, &[], expected_tokens);
    }

    fn run_test_with_defines(source: &str, defines: &[(&str, &str)], expected_tokens: &[&[&str]]) {
        unsafe {
            let lexer = ffi::lexer_create();

            let defines_htab = ffi::tvm_htab_create();

            for pair in defines.iter() {
                ffi::tvm_htab_add_ref(
                    defines_htab,
                    CString::new(pair.0).unwrap().as_ptr(),
                    CString::new(pair.1).unwrap().as_ptr().cast(),
                    (pair.1.len() + 1) as i32,
                );
            }

            let source = CString::new(source).unwrap().into_raw();
            ffi::tvm_lex(lexer, source, defines_htab);
            drop(CString::from_raw(source));

            let mut actual = Vec::<Vec<&str>>::default();

            let mut line_index = 0;
            loop {
                let line_pointer = *(*lexer).tokens.offset(line_index as isize);
                if line_pointer.is_null() {
                    break;
                }

                let mut current_line = Vec::<&str>::default();
                let mut token_index = 0;
                loop {
                    let token_pointer = *line_pointer.offset(token_index as isize);
                    if token_pointer.is_null() {
                        break;
                    }

                    current_line.push(CStr::from_ptr(token_pointer).to_str().unwrap());

                    token_index = token_index + 1
                }

                actual.push(current_line);

                line_index = line_index + 1;
            }

            assert_eq!(actual, expected_tokens);

            ffi::tvm_lexer_destroy(lexer);
        }
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
}
