use super::{
    line_parser::{ParsedLine, ParsedLineInstruction},
    unresolved_instruction::{UnresolvedInstruction, UnresolvedSource},
};
use crate::instruction::{Instruction, Source};
use std::collections::{hash_map::Entry, HashMap};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ResolveError<'a> {
    UndefinedLabel(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ParseLabelsError<'a> {
    DuplicateDefinition(&'a str),
}

pub(super) fn resolve<'a>(
    instruction: &UnresolvedInstruction<'a>,
    labels: &HashMap<&str, i32>,
) -> Result<Instruction, ResolveError<'a>> {
    macro_rules! resolve {
        ($value:expr) => {
            match $value {
                UnresolvedSource::Register(reg) => Source::Register(*reg),
                UnresolvedSource::Value(value) => Source::Value(*value),
                UnresolvedSource::Address(address) => Source::Address(*address),
                UnresolvedSource::Label(label) => {
                    let value = labels
                        .get(label)
                        .ok_or(ResolveError::UndefinedLabel(label))?;
                    Source::Value(*value)
                }
            }
        };
    }

    let result = match instruction {
        UnresolvedInstruction::Nop => Instruction::Nop,
        UnresolvedInstruction::Int => Instruction::Int,
        UnresolvedInstruction::Mov(target, source) => Instruction::Mov(*target, resolve!(source)),
        UnresolvedInstruction::Push(source) => Instruction::Push(resolve!(source)),
        UnresolvedInstruction::Pop(target) => Instruction::Pop(*target),
        UnresolvedInstruction::Pushf => Instruction::Pushf,
        UnresolvedInstruction::Popf => Instruction::Popf,
        UnresolvedInstruction::Inc(target) => Instruction::Inc(*target),
        UnresolvedInstruction::Dec(target) => Instruction::Dec(*target),
        UnresolvedInstruction::Add(target, source) => Instruction::Add(*target, resolve!(source)),
        UnresolvedInstruction::Sub(target, source) => Instruction::Sub(*target, resolve!(source)),
        UnresolvedInstruction::Mul(target, source) => Instruction::Mul(*target, resolve!(source)),
        UnresolvedInstruction::Div(target, source) => Instruction::Div(*target, resolve!(source)),
        UnresolvedInstruction::Mod(source1, source2) => {
            Instruction::Mod(resolve!(source1), resolve!(source2))
        }
        UnresolvedInstruction::Rem(target) => Instruction::Rem(*target),
        UnresolvedInstruction::Not(target) => Instruction::Not(*target),
        UnresolvedInstruction::Xor(target, source) => Instruction::Xor(*target, resolve!(source)),
        UnresolvedInstruction::Or(target, source) => Instruction::Or(*target, resolve!(source)),
        UnresolvedInstruction::And(target, source) => Instruction::And(*target, resolve!(source)),
        UnresolvedInstruction::Shl(target, source) => Instruction::Shl(*target, resolve!(source)),
        UnresolvedInstruction::Shr(target, source) => Instruction::Shr(*target, resolve!(source)),
        UnresolvedInstruction::Cmp(source1, source2) => {
            Instruction::Cmp(resolve!(source1), resolve!(source2))
        }
        UnresolvedInstruction::Jmp(source) => Instruction::Jmp(resolve!(source)),
        UnresolvedInstruction::Call(source) => Instruction::Call(resolve!(source)),
        UnresolvedInstruction::Ret => Instruction::Ret,
        UnresolvedInstruction::Je(source) => Instruction::Je(resolve!(source)),
        UnresolvedInstruction::Jne(source) => Instruction::Jne(resolve!(source)),
        UnresolvedInstruction::Jg(source) => Instruction::Jg(resolve!(source)),
        UnresolvedInstruction::Jge(source) => Instruction::Jge(resolve!(source)),
        UnresolvedInstruction::Jl(source) => Instruction::Jl(resolve!(source)),
        UnresolvedInstruction::Jle(source) => Instruction::Jle(resolve!(source)),
        UnresolvedInstruction::Prn(source) => Instruction::Prn(resolve!(source)),
    };

    Ok(result)
}

pub(super) fn resolve_labels<'a>(
    lines: &[ParsedLine<'a>],
) -> Result<HashMap<&'a str, usize>, ParseLabelsError<'a>> {
    let mut labels = HashMap::<&'a str, usize>::default();

    let mut instruction_index: usize = 0;

    for line in lines {
        for label in &line.labels {
            match labels.entry(label) {
                Entry::Occupied(occupied) => {
                    return Err(ParseLabelsError::DuplicateDefinition(*occupied.key()))
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

#[cfg(test)]
mod tests {
    use super::super::line_parser::parse_line;
    use super::*;
    use crate::{htab::HashTable, lexer::LexerContext};

    fn run(source: &str, expected_labels: Option<&[(&str, usize)]>) {
        let lexer = LexerContext::lex(source, &HashTable::default());
        let lines: Vec<_> = lexer
            .tokens()
            .iter()
            .map(|line| {
                let tokens: Vec<_> = line.iter().map(|token| token.to_str().unwrap()).collect();
                parse_line(&tokens)
            })
            .collect();

        let result = resolve_labels(&lines);

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
    fn resolve_labels_can_resolve_labels() {
        run("label1: add eax, ebx\nstart: inc ebx \n\ndec eax\nlabel2: sub eax, ebx\nlabel3:\nlabel4:\ninc eax", Some(&[("label1", 0), ("start", 1), ("label2", 3), ("label3", 4), ("label4", 4)]));
    }

    #[test]
    fn resolve_labels_returns_if_a_label_is_defined_twice() {
        run("label1: add eax, ebx\nlabel1: inc ebx", None);
    }

    #[test]
    fn resolve_labels_ignroes_invalid_instructions_when_counting_instructions() {
        run("add eax, ebx\n\nbad\nlabel: inc ebx", Some(&[("label", 1)]))
    }
}
