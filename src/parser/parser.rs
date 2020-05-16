use super::{
    line_parser::{parse_line, ParsedLine, ParsedLineInstruction},
    resolver::resolve,
};
use crate::{context::Program, instruction::Instruction};
use std::collections::{hash_map::Entry, HashMap};

pub(super) fn is_valid_label(s: &str) -> bool {
    fn is_valid_first_char(c: u8) -> bool {
        match c {
            b'$' | b'@' | b'_' => true,
            b'A'..=b'Z' => true,
            b'a'..=b'z' => true,
            _ => false,
        }
    }

    fn is_valid_char(c: u8) -> bool {
        match c {
            b'0'..=b'9' => true,
            _ => is_valid_first_char(c),
        }
    }

    let mut bytes = s.bytes();
    if let Some(first) = bytes.next() {
        is_valid_first_char(first) && bytes.all(is_valid_char)
    } else {
        false
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ParseErrorKind {
    DuplicateLabel(String),
    UndefinedLabel(String),
    InvalidInstruction(String),
    MissingOperand(usize),
    InvalidOperand(String),
    ExtraToken(String),
}

#[derive(Debug, PartialEq)]
pub struct ParseError {
    line_index: usize,
    error: ParseErrorKind,
}

fn gather_label_values<'a>(lines: &[ParsedLine<'a>]) -> Result<HashMap<&'a str, i32>, ParseError> {
    let mut instruction_index: i32 = 0;

    let mut labels = HashMap::<&str, i32>::default();
    for (line_index, line) in lines.iter().enumerate() {
        for label in &line.labels {
            match labels.entry(label) {
                Entry::Occupied(occupied) => {
                    return Err(ParseError {
                        line_index,
                        error: ParseErrorKind::DuplicateLabel((*occupied.key()).to_owned()),
                    });
                }
                Entry::Vacant(vacant) => vacant.insert(instruction_index),
            };
        }

        if let ParsedLineInstruction::Some(_) = line.instruction {
            instruction_index += 1;
        }
    }

    Ok(labels)
}

pub(crate) fn parse<'a>(lines: &[Vec<&'a str>]) -> Result<Program, ParseError> {
    let parsed_lines: Vec<_> = lines.iter().map(|l| parse_line(l)).collect();

    let labels = gather_label_values(&parsed_lines)?;

    let mut instructions: Vec<Instruction> = vec![];

    for (line_index, line) in parsed_lines.iter().enumerate() {
        match &line.instruction {
            ParsedLineInstruction::Some(instruction) => {
                let resolved = resolve(instruction, &labels);
                match resolved {
                    Ok(i) => instructions.push(i),
                    Err(e) => {
                        return Err(ParseError {
                            line_index,
                            error: e,
                        })
                    }
                };
            }
            ParsedLineInstruction::None => {}
            ParsedLineInstruction::Err(e) => {
                return Err(ParseError {
                    line_index,
                    error: e.clone(),
                });
            }
        }
    }

    let start_instruction_index = labels.get("start").map(|v| *v as i32).unwrap_or(0);

    let program = Program {
        instructions,
        start_instruction_index,
    };

    Ok(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::{Instruction, Register, Source, Target};
    use crate::{htab::HashTable, lexer::LexerContext};

    #[test]
    fn can_parse_with_resolved_labels() {
        let htab = HashTable::default();
        let lexer = LexerContext::lex("label1: add eax, ebx\njmp label4\nstart: inc ebx \n\ndec eax\nlabel2: sub eax, label1\nlabel3:\nlabel4:\ninc eax\njmp start\njmp label3", &htab);
        let result = parse(&lexer.tokens()).unwrap();

        assert_eq!(
            result.instructions,
            &[
                Instruction::Add(
                    Target::Register(Register::Eax),
                    Source::Register(Register::Ebx)
                ),
                Instruction::Jmp(Source::Value(5)),
                Instruction::Inc(Target::Register(Register::Ebx)),
                Instruction::Dec(Target::Register(Register::Eax)),
                Instruction::Sub(Target::Register(Register::Eax), Source::Value(0)),
                Instruction::Inc(Target::Register(Register::Eax)),
                Instruction::Jmp(Source::Value(2)),
                Instruction::Jmp(Source::Value(5)),
            ]
        );
    }

    #[test]
    fn start_instruction_index_is_set_to_the_start_label() {
        let htab = HashTable::default();
        let lexer = LexerContext::lex("label1: add eax, ebx\njmp label4\nstart: inc ebx \n\ndec eax\nlabel2: sub eax, label1\nlabel3:\nlabel4:\ninc eax\njmp start\njmp label3", &htab);
        let result = parse(&lexer.tokens()).unwrap();

        assert_eq!(result.start_instruction_index, 2);
    }

    #[test]
    fn start_instruction_index_is_zero_if_no_start_label_exists() {
        let htab = HashTable::default();
        let lexer = LexerContext::lex("label1: add eax, ebx\njmp label4\ninc ebx \n\ndec eax\nlabel2: sub eax, label1\nlabel3:\nlabel4:\ninc eax", &htab);
        let result = parse(&lexer.tokens()).unwrap();

        assert_eq!(result.start_instruction_index, 0);
    }

    #[test]
    fn returns_duplicate_definition_error_if_a_label_is_defined_twice() {
        let htab = HashTable::default();
        let lexer = LexerContext::lex(
            "label1: add eax, ebx\n\nlabel1: inc ebx\nlabel2: dec eax",
            &htab,
        );
        let result = parse(&lexer.tokens());

        match result {
            Err(e) => {
                assert_eq!(e.line_index, 2);
                assert_eq!(e.error, ParseErrorKind::DuplicateLabel("label1".to_owned()));
            }
            Ok(_) => panic!("Expected failure"),
        }
    }

    #[test]
    fn returns_undefined_error_if_a_label_is_not_defined() {
        let htab = HashTable::default();
        let lexer = LexerContext::lex("inc eax\n\njmp label1", &htab);
        let result = parse(&lexer.tokens());

        match result {
            Err(e) => {
                assert_eq!(e.line_index, 2);
                assert_eq!(e.error, ParseErrorKind::UndefinedLabel("label1".to_owned()));
            }
            Ok(_) => panic!("Expected failure"),
        }
    }

    #[test]
    fn parse_errors_are_correctly_returned() {
        let htab = HashTable::default();
        let lexer = LexerContext::lex("inc eax\n\nbad", &htab);
        let result = parse(&lexer.tokens());

        match result {
            Err(e) => {
                assert_eq!(e.line_index, 2);
                assert_eq!(
                    e.error,
                    ParseErrorKind::InvalidInstruction("bad".to_owned())
                );
            }
            Ok(_) => panic!("Expected failure"),
        }
    }
}
