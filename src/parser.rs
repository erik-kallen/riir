#[cfg(test)]
mod tests {
    mod parse_labels {
        use crate::{context::Context, ffi, htab::HashTable, lexer::LexerContext};
        use std::ffi::CString;

        fn run(source: &str, expected_labels: Option<&[(&str, usize)]>) {
            run_ffi(source, expected_labels);
        }

        fn run_ffi(source: &str, expected_labels: Option<&[(&str, usize)]>) {
            unsafe {
                let lexer = LexerContext::lex(source, &HashTable::default());
                let mut vm = Context::new();
                let result = ffi::tvm_parse_labels(&mut vm as *mut _ as *mut _, lexer.tokens_ptr);

                match expected_labels {
                    None => {
                        assert_eq!(result, 1);
                    }
                    Some(expected_labels) => {
                        assert_eq!(result, 0);
                        assert_eq!(vm.program.labels.0.len(), expected_labels.len());

                        for expected_label in expected_labels {
                            assert_eq!(
                                ffi::tvm_htab_find(
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
        fn can_parse_labels() {
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
                let result = ffi::tvm_parse_labels(&mut vm as *mut _ as *mut _, lexer.tokens_ptr);

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
        use crate::{
            context::Context, ffi, htab::HashTable, instruction::OpCode, lexer::LexerContext,
        };
        use std::{
            os::raw::{c_char, c_int},
            ptr::null_mut,
            slice,
        };

        #[test]
        fn can_parse_program() {
            let lexer = LexerContext::lex(
                "jmp label\nmov eax, 100\nlabel: inc ebx\nadd eax, 101",
                &HashTable::default(),
            );
            let mut vm = Context::new();

            unsafe {
                ffi::tvm_htab_add(
                    vm.program.label_htab_ptr,
                    b"label\0".as_ptr() as *const c_char,
                    2,
                );

                let result = ffi::tvm_parse_program(&mut vm as *mut _ as *mut _, lexer.tokens_ptr);

                assert_eq!(result, 0);

                assert_eq!(
                    slice::from_raw_parts(
                        vm.program.instructions,
                        vm.program.num_instructions as usize + 1
                    ),
                    [
                        OpCode::Jmp as c_int,
                        OpCode::Mov as c_int,
                        OpCode::Inc as c_int,
                        OpCode::Add as c_int,
                        -1 as c_int
                    ]
                );

                assert_eq!(vm.program.num_values, 3);
                assert_eq!(**vm.program.values.offset(0), 2);
                assert_eq!(**vm.program.values.offset(1), 100);
                assert_eq!(**vm.program.values.offset(2), 101);

                assert_eq!(
                    slice::from_raw_parts(*vm.program.args.offset(0), ffi::MAX_ARGS as usize),
                    [*vm.program.values, null_mut()]
                );
                assert_eq!(
                    slice::from_raw_parts(*vm.program.args.offset(1), ffi::MAX_ARGS as usize),
                    [
                        &vm.memory.registers[0] as *const _ as *mut c_int,
                        *vm.program.values.offset(1),
                    ]
                );
                assert_eq!(
                    slice::from_raw_parts(*vm.program.args.offset(2), ffi::MAX_ARGS as usize),
                    [
                        &vm.memory.registers[1] as *const _ as *mut c_int,
                        null_mut(),
                    ]
                );
                assert_eq!(
                    slice::from_raw_parts(*vm.program.args.offset(3), ffi::MAX_ARGS as usize),
                    [
                        &vm.memory.registers[0] as *const _ as *mut c_int,
                        *vm.program.values.offset(2),
                    ]
                );
            }
        }
    }
}
