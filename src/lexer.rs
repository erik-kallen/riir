use crate::{ffi, htab::HashTable};
use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    ptr::{null, null_mut},
};

#[repr(C)]
pub struct LexerContext {
    // Next are what the C interface expects
    source_lines_ptr: *const *const c_char,
    pub tokens_ptr: *mut *mut *const c_char,

    // Next are storage for the pointers we need for the C interface
    tokens_ptr_storage_1: Vec<Vec<*const c_char>>,
    tokens_ptr_storage_2: Vec<*mut *const c_char>,

    // Next are real data
    tokens: Vec<Vec<CString>>,
}

fn lex(source: &str, defines: &HashTable) -> Vec<Vec<CString>> {
    source
        .lines()
        .map(|line| {
            // Ignore everything after comment
            let line = match line.find("#") {
                Some(i) => line.split_at(i).0,
                None => line,
            };

            let line_tokens: Vec<CString> = line
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

                    CString::new(token).unwrap()
                })
                .collect();

            line_tokens
        })
        .collect()
}

impl LexerContext {
    fn empty() -> LexerContext {
        LexerContext {
            source_lines_ptr: null(),
            tokens_ptr: null_mut(),
            tokens_ptr_storage_1: vec![],
            tokens_ptr_storage_2: vec![],
            tokens: vec![],
        }
    }

    fn lex_into_self(self: &mut LexerContext, source: &str, defines: &HashTable) {
        self.tokens = lex(source, defines);

        self.tokens_ptr_storage_1 = self
            .tokens
            .iter()
            .map(|line| {
                let mut line_vec: Vec<*const c_char> =
                    line.iter().map(|token| token.as_ptr()).collect();

                // A little strange that we need to do this, but the parser will assume that it can use this many tokens, even if there is a null token before it
                while line_vec.len() < ffi::MAX_TOKENS as usize {
                    line_vec.push(null());
                }
                line_vec.push(null());
                line_vec
            })
            .collect();

        self.tokens_ptr_storage_2 = self
            .tokens_ptr_storage_1
            .iter_mut()
            .map(|line| line.as_mut_ptr())
            .collect();

        self.tokens_ptr_storage_2.push(null_mut());

        self.tokens_ptr = self.tokens_ptr_storage_2.as_mut_ptr();
    }
}

#[no_mangle]
pub unsafe extern "C" fn lexer_create() -> *mut ffi::tvm_lexer_ctx {
    let result = Box::new(LexerContext::empty());

    Box::into_raw(result).cast()
}

#[no_mangle]
pub unsafe extern "C" fn tvm_lexer_destroy(l: *mut ffi::tvm_lexer_ctx) {
    if l.is_null() {
        return;
    }

    let l = Box::from_raw(l as *mut LexerContext);
    drop(l);
}

#[no_mangle]
pub unsafe extern "C" fn tvm_lex(
    lexer: *mut ffi::tvm_lexer_ctx,
    source: *mut c_char,
    defines: *mut ffi::tvm_htab_ctx,
) {
    let lexer = &mut *(lexer as *mut LexerContext);
    let defines = Box::from_raw(defines as *mut HashTable); // For some reason we take ownership of the defines and should drop it after lexing
    let source = CStr::from_ptr(source).to_str().unwrap();

    lexer.lex_into_self(source, &*defines);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ffi,
        htab::{HashTable, Item},
    };
    use std::ffi::{CStr, CString};

    fn run_test(source: &str, expected_tokens: &[&[&str]]) {
        run_test_with_defines(source, &[], expected_tokens);
    }

    fn run_test_with_defines(source: &str, defines: &[(&str, &str)], expected_tokens: &[&[&str]]) {
        run_test_with_defines_in_rust(source, defines, expected_tokens);
        run_test_with_defines_against_ffi(source, defines, expected_tokens);
    }

    fn run_test_with_defines_in_rust(
        source: &str,
        defines: &[(&str, &str)],
        expected_tokens: &[&[&str]],
    ) {
        let mut defines_htab = HashTable::default();
        for define in defines {
            defines_htab.0.insert(
                CString::new(define.0).unwrap(),
                Item::opaque(CString::new(define.1).unwrap()),
            );
        }

        let actual: Vec<Vec<String>> = lex(source, &defines_htab)
            .iter()
            .map(|line| {
                line.iter()
                    .map(|token| String::from(token.to_str().unwrap()))
                    .collect()
            })
            .collect();

        assert_eq!(actual, expected_tokens);
    }

    fn run_test_with_defines_against_ffi(
        source: &str,
        defines: &[(&str, &str)],
        expected_tokens: &[&[&str]],
    ) {
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
