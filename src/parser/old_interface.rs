use super::{
    line_parser::{parse_line, ParsedLineInstruction},
    resolver::{resolve, resolve_labels},
};
use crate::{context::Context, htab::Item};
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    os::raw::{c_char, c_int, c_void},
    pin::Pin,
};

unsafe fn tvm_build_lines_vec<'a>(tokens: *mut *mut *const c_char) -> Vec<Vec<&'a str>> {
    let mut lines_vec = Vec::<Vec<&str>>::default();

    let mut current_line_pointer = tokens;
    loop {
        let current_line = *current_line_pointer;

        if current_line.is_null() {
            break;
        }

        let mut current_line_vec = Vec::<&str>::default();

        let mut current_token_pointer = current_line;
        loop {
            let current_token = *current_token_pointer;
            if current_token.is_null() {
                break;
            }

            let current_token_str = CStr::from_ptr(current_token).to_str().unwrap();
            current_line_vec.push(current_token_str);

            current_token_pointer = current_token_pointer.offset(1);
        }

        lines_vec.push(current_line_vec);

        current_line_pointer = current_line_pointer.offset(1);
    }

    lines_vec
}

#[no_mangle]
pub unsafe extern "C" fn tvm_parse_labels(
    vm: *mut c_void,
    tokens: *mut *mut *const c_char,
) -> c_int {
    let vm = &mut *(vm as *mut Context);

    let lines_vec = tvm_build_lines_vec(tokens);
    let parsed_lines: Vec<_> = lines_vec.iter().map(|l| parse_line(l)).collect();

    let parse_result = resolve_labels(&parsed_lines);
    if let Ok(labels) = parse_result {
        let program = Pin::get_unchecked_mut(Pin::as_mut(&mut vm.program));

        if let Some(start) = labels.get("start") {
            program.start_instruction_index = *start as i32;
        }

        for (key, value) in labels {
            program
                .labels
                .0
                .insert(CString::new(key).unwrap(), Item::integer(value as i32));
        }

        0
    } else {
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn tvm_parse_program(
    vm: *mut c_void,
    tokens: *mut *mut *const c_char,
) -> c_int {
    let vm = &mut *(vm as *mut Context);

    let lines_vec = tvm_build_lines_vec(tokens);

    let instructions: Vec<_> = lines_vec
        .iter()
        .filter_map(|line| match parse_line(&line).instruction {
            ParsedLineInstruction::Some(instruction) => Some(instruction),
            _ => None,
        })
        .collect();

    let labels: HashMap<&str, i32> = vm
        .program
        .labels
        .0
        .iter()
        .map(|p| (p.0.to_str().unwrap(), p.1.value()))
        .collect();

    let instructions: Result<Vec<_>, _> =
        instructions.iter().map(|i| resolve(i, &labels)).collect();

    if let Ok(instructions) = instructions {
        let program = Pin::get_unchecked_mut(Pin::as_mut(&mut vm.program));
        program.instructions = instructions;

        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    mod parse_labels {
        use super::super::*;
        use crate::{
            context::Context,
            htab::{tvm_htab_find, HashTable},
            lexer::LexerContext,
        };
        use std::ffi::CString;

        fn run(source: &str, expected_labels: Option<&[(&str, usize)]>) {
            unsafe {
                let lexer = LexerContext::lex(source, &HashTable::default());
                let mut vm = Context::new();
                let result = tvm_parse_labels(&mut vm as *mut _ as *mut _, lexer.tokens_ptr);

                match expected_labels {
                    None => {
                        assert_eq!(result, 1);
                    }
                    Some(expected_labels) => {
                        assert_eq!(result, 0);
                        assert_eq!(vm.program.labels.0.len(), expected_labels.len());

                        for expected_label in expected_labels {
                            assert_eq!(
                                tvm_htab_find(
                                    vm.program.label_htab_ptr,
                                    CString::new(expected_label.0).unwrap().as_ptr()
                                ),
                                expected_label.1 as i32
                            );
                        }
                    }
                };
            }
        }

        #[test]
        fn parse_labels_can_parse_labels() {
            run("label1: add eax, ebx\nstart: inc ebx \n\ndec eax\nlabel2: sub eax, ebx\nlabel3:\nlabel4:\ninc eax", Some(&[("label1", 0), ("start", 1), ("label2", 3), ("label3", 4), ("label4", 4)]));
        }

        #[test]
        fn ffi_interface_sets_the_program_start_instruction_index_correctly() {
            unsafe {
                let lexer = LexerContext::lex(
                    "label1: add eax, ebx\ninc eax\nstart: inc ebx \ndec eax",
                    &HashTable::default(),
                );
                let mut vm = Context::new();
                let result = tvm_parse_labels(&mut vm as *mut _ as *mut _, lexer.tokens_ptr);

                assert_eq!(result, 0);
                assert_eq!(vm.program.start_instruction_index, 2)
            }
        }

        #[test]
        fn an_error_is_returned_if_a_label_is_defined_twice() {
            run("label1: add eax, ebx\nlabel1: inc ebx", None);
        }

        #[test]
        fn invalid_instructions_are_ignored_when_counting_instructions() {
            run("add eax, ebx\n\nbad\nlabel: inc ebx", Some(&[("label", 1)]))
        }
    }

    mod parse_program {
        use super::super::*;
        use crate::{
            context::Context,
            htab::{tvm_htab_add, HashTable},
            instruction::{Instruction, Register, Source, Target},
            lexer::LexerContext,
        };
        use std::os::raw::c_char;

        #[test]
        fn parse_program_works() {
            let lexer = LexerContext::lex(
                "jmp label\nmov eax, 100\nlabel: inc ebx\nadd eax, 101",
                &HashTable::default(),
            );
            let mut vm = Context::new();

            unsafe {
                tvm_htab_add(
                    vm.program.label_htab_ptr,
                    b"label\0".as_ptr() as *const c_char,
                    2,
                );

                let result = tvm_parse_program(&mut vm as *mut _ as *mut _, lexer.tokens_ptr);

                assert_eq!(result, 0);

                assert_eq!(
                    vm.program.instructions,
                    vec!(
                        Instruction::Jmp(Source::Value(2)),
                        Instruction::Mov(Target::Register(Register::Eax), Source::Value(100)),
                        Instruction::Inc(Target::Register(Register::Ebx)),
                        Instruction::Add(Target::Register(Register::Eax), Source::Value(101)),
                    )
                );
            }
        }
    }
}
