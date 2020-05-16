use super::{
    unresolved_instruction::{UnresolvedInstruction, UnresolvedSource},
    ParseErrorKind,
};
use crate::instruction::{Instruction, Source};
use std::collections::HashMap;

pub(super) fn resolve<'a>(
    instruction: &UnresolvedInstruction<'a>,
    labels: &HashMap<&str, i32>,
) -> Result<Instruction, ParseErrorKind> {
    macro_rules! resolve {
        ($value:expr) => {
            match $value {
                UnresolvedSource::Register(reg) => Source::Register(*reg),
                UnresolvedSource::Value(value) => Source::Value(*value),
                UnresolvedSource::Address(address) => Source::Address(*address),
                UnresolvedSource::Label(label) => {
                    let value = labels
                        .get(label)
                        .ok_or(ParseErrorKind::UndefinedLabel((*label).to_owned()))?;
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
