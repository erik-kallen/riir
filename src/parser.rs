use crate::{
    context::Context, htab::Item, instruction::{OpCode, Operand},
};
use std::{
    collections::{hash_map::Entry, HashMap},
    ffi::{CStr, CString},
    mem::forget,
    os::raw::{c_char, c_int, c_void},
    pin::Pin,
    ptr,
};

#[derive(Debug)]
pub enum ParseLabelsError {
    DuplicateDefinition(String),
}

#[derive(Debug, PartialEq)]
enum OperandOrLabel<'a> {
    Operand(Operand),
    Label(&'a str),
}

#[derive(Debug, PartialEq)]
enum ParsedLineInstruction<'a> {
    Some(OpCode, Vec<OperandOrLabel<'a>>),
    None,
    Err(ParseLineError<'a>),
}

#[derive(Debug, PartialEq)]
struct ParsedLine<'a> {
    labels: Vec<&'a str>,
    instruction: ParsedLineInstruction<'a>,
}

#[derive(Debug, PartialEq)]
enum ParseLineError<'a> {
    UnexpectedToken(&'a str),
    InvalidInstruction(&'a str),
}

pub const NUM_ARGS: usize = 2;

pub fn parse_labels<'a>(
    lines: &'a [Vec<&'a str>],
) -> Result<HashMap<&'a str, usize>, ParseLabelsError> {
    let mut labels = HashMap::<&'a str, usize>::default();

    let mut instruction_index: usize = 0;

    for line in lines {
        let parsed_line = parse_line(&line);

        for label in parsed_line.labels {
            match labels.entry(label) {
                Entry::Occupied(occupied) => {
                    return Err(ParseLabelsError::DuplicateDefinition(
                        (*occupied.key()).to_owned(),
                    ))
                }
                Entry::Vacant(vacant) => vacant.insert(instruction_index),
            };
        }

        if let ParsedLineInstruction::Some(_, _) = parsed_line.instruction {
            instruction_index += 1;
        }
    }

    Ok(labels)
}

fn parse_line<'a>(tokens: &[&'a str]) -> ParsedLine<'a> {
    let mut labels = Vec::<&str>::default();
    let mut opcode: Option<OpCode> = None;
    let mut operands = Vec::<OperandOrLabel<'a>>::default();

    for token in tokens {
        if token.ends_with(':') {
            if opcode.is_some() {
                return ParsedLine {
                    labels,
                    instruction: ParsedLineInstruction::Err(ParseLineError::UnexpectedToken(token)),
                };
            }
            labels.push(&token[0..token.len() - 1]);
        } else {
            if opcode.is_none() {
                opcode = OpCode::parse(token);
                if opcode.is_none() {
                    return ParsedLine {
                        labels,
                        instruction: ParsedLineInstruction::Err(
                            ParseLineError::InvalidInstruction(token),
                        ),
                    };
                };
            } else {
                let operand = match Operand::parse(token) {
                    Some(o) => OperandOrLabel::Operand(o),
                    None => OperandOrLabel::Label(token),
                };
                operands.push(operand);
            }
        }
    }

    ParsedLine {
        labels,
        instruction: if let Some(i) = opcode {
            ParsedLineInstruction::Some(i, operands)
        } else {
            ParsedLineInstruction::None
        },
    }
}

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

    let parse_result = parse_labels(&lines_vec);
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
            ParsedLineInstruction::Some(instruction, operands) => Some((instruction, operands)),
            _ => None,
        })
        .collect();

    let program = Pin::get_unchecked_mut(Pin::as_mut(&mut vm.program));

    // Allocate and populate instructions
    let mut instructions_vec: Vec<c_int> = Vec::with_capacity(instructions.len() + 1);

    instructions_vec.extend(instructions.iter().map(|inst| inst.0 as c_int));
    instructions_vec.push(-1);

    // Allocate and populate args
    let mut args: Vec<*mut *mut c_int> = Vec::with_capacity(instructions.len() + 1);

    let mut values: Vec<*mut c_int> = Vec::default();

    for instruction in instructions.iter() {
        let mut current_args: Vec<*mut c_int> = vec![ptr::null_mut(); NUM_ARGS];

        for (index, operand) in instruction.1.iter().enumerate() {
            unsafe fn add_value(values: &mut Vec<*mut c_int>, value: c_int) -> *mut c_int {
                let pointer = Box::into_raw(Box::new(value));
                values.push(pointer);
                pointer
            }

            let pointer = match operand {
                OperandOrLabel::Operand(Operand::Register(reg)) => {
                    vm.memory.registers.as_ptr().offset(*reg as isize) as *mut c_int
                },
                OperandOrLabel::Operand(Operand::Address(addr)) => {
                    (vm.memory.mem_space_ptr as *mut c_int).offset(*addr as isize)
                },
                OperandOrLabel::Operand(Operand::Value(value)) => add_value(&mut values, *value),
                OperandOrLabel::Label(label) => {
                    let value = program
                        .labels
                        .0
                        .get(&CString::new(*label).unwrap())
                        .map_or(0, |i| i.value());

                    add_value(&mut values, value)
                },
            };

            current_args[index] = pointer;
        }

        args.push(current_args.as_mut_ptr());

        forget(current_args);
    }

    args.push(ptr::null_mut());

    program.instructions = instructions_vec.as_mut_ptr();
    program.args = args.as_mut_ptr();

    values.shrink_to_fit();
    program.values = values.as_mut_ptr();
    program.num_values = values.capacity() as c_int;

    program.num_instructions = instructions.len() as c_int;

    forget(instructions_vec);
    forget(args);
    forget(values);

    0
}

#[cfg(test)]
mod tests {
    mod parse_labels {
        use super::super::*;
        use crate::{context::Context, htab::{HashTable, tvm_htab_find}, lexer::LexerContext};
        use std::ffi::CString;

        fn run(source: &str, expected_labels: Option<&[(&str, usize)]>) {
            run_ffi(source, expected_labels);
            run_rust(source, expected_labels);
        }

        fn run_ffi(source: &str, expected_labels: Option<&[(&str, usize)]>) {
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

        fn run_rust(source: &str, expected_labels: Option<&[(&str, usize)]>) {
            let lexer = LexerContext::lex(source, &HashTable::default());
            let tokens: Vec<Vec<&str>> = lexer
                .tokens()
                .iter()
                .map(|line| line.iter().map(|token| token.to_str().unwrap()).collect())
                .collect();

            let result = parse_labels(&tokens);

            match expected_labels {
                None => {
                    assert!(result.is_err());
                }
                Some(expected_labels) => {
                    let actual_labels = result.unwrap();
                    assert_eq!(actual_labels.len(), expected_labels.len());
                    for expected_label in expected_labels {
                        assert_eq!(
                            *actual_labels.get(expected_label.0).unwrap(),
                            expected_label.1
                        );
                    }
                }
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
        use crate::{
            context::Context,
            htab::{HashTable, tvm_htab_add},
            instruction::{OpCode, Register},
            lexer::LexerContext,
        };
        use std::{
            os::raw::{c_char, c_int},
            ptr::null_mut,
            slice,
        };
        use super::super::*;

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
                    slice::from_raw_parts(*vm.program.args.offset(0), NUM_ARGS as usize),
                    [*vm.program.values, null_mut()]
                );
                assert_eq!(
                    slice::from_raw_parts(*vm.program.args.offset(1), NUM_ARGS as usize),
                    [
                        &vm.memory.registers[Register::Eax as usize] as *const _ as *mut c_int,
                        *vm.program.values.offset(1),
                    ]
                );
                assert_eq!(
                    slice::from_raw_parts(*vm.program.args.offset(2), NUM_ARGS as usize),
                    [
                        &vm.memory.registers[Register::Ebx as usize] as *const _ as *mut c_int,
                        null_mut(),
                    ]
                );
                assert_eq!(
                    slice::from_raw_parts(*vm.program.args.offset(3), NUM_ARGS as usize),
                    [
                        &vm.memory.registers[Register::Eax as usize] as *const _ as *mut c_int,
                        *vm.program.values.offset(2),
                    ]
                );
            }
        }
    }

    mod parse_line {
        use super::super::{ParsedLineInstruction::*, *};
        use crate::instruction::{Operand, Register};

        fn run(
            tokens: &[&str],
            expected_labels: &[&str],
            expected_instruction: ParsedLineInstruction,
        ) {
            assert_eq!(
                parse_line(tokens),
                ParsedLine {
                    labels: expected_labels.to_owned(),
                    instruction: expected_instruction
                }
            );
        }

        #[test]
        fn can_parse_empty_line() {
            run(&[], &[], None);
        }

        #[test]
        fn can_parse_line_with_only_instruction() {
            run(&["nop"], &[], Some(OpCode::Nop, vec![]));
        }

        #[test]
        fn can_parse_line_with_instruction_and_operands() {
            run(
                &["inc", "eax"],
                &[],
                Some(
                    OpCode::Inc,
                    vec![OperandOrLabel::Operand(Operand::Register(Register::Eax))],
                ),
            );
            run(
                &["add", "ebx", "1"],
                &[],
                Some(
                    OpCode::Add,
                    vec![
                        OperandOrLabel::Operand(Operand::Register(Register::Ebx)),
                        OperandOrLabel::Operand(Operand::Value(1)),
                    ],
                ),
            );
            run(
                &["jmp", "label"],
                &[],
                Some(OpCode::Jmp, vec![OperandOrLabel::Label("label")]),
            );
        }

        #[test]
        fn can_parse_line_with_only_labels() {
            run(&["label1:"], &["label1"], None);
            run(&["label1:", "label2:"], &["label1", "label2"], None);
        }

        #[test]
        fn can_parse_line_with_labels_and_instruction() {
            run(
                &["label1:", "nop"],
                &["label1"],
                Some(OpCode::Nop, vec![]),
            );
            run(
                &["label1:", "label2:", "nop"],
                &["label1", "label2"],
                Some(OpCode::Nop, vec![]),
            );

            run(
                &["label1:", "inc", "eax"],
                &["label1"],
                Some(
                    OpCode::Inc,
                    vec![OperandOrLabel::Operand(Operand::Register(Register::Eax))],
                ),
            );
            run(
                &["label1:", "label2:", "inc", "eax"],
                &["label1", "label2"],
                Some(
                    OpCode::Inc,
                    vec![OperandOrLabel::Operand(Operand::Register(Register::Eax))],
                ),
            );
        }

        #[test]
        fn errors_are_correctly_reported() {
            run(
                &["nop", "label1:"],
                &[],
                Err(ParseLineError::UnexpectedToken("label1:")),
            );
            run(
                &["bad"],
                &[],
                Err(ParseLineError::InvalidInstruction("bad")),
            );
        }

        #[test]
        fn labels_are_returned_for_lines_with_errors() {
            run(
                &["label1:", "nop", "label2:"],
                &["label1"],
                Err(ParseLineError::UnexpectedToken("label2:")),
            );
            run(
                &["label1:", "bad"],
                &["label1"],
                Err(ParseLineError::InvalidInstruction("bad")),
            );
        }
    }
}
