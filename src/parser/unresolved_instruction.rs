use super::{is_valid_label, register::parse_register, ParseErrorKind};
use crate::instruction::{Register, Target};
use std::num::ParseIntError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum UnresolvedSource<'a> {
    Register(Register),
    Value(i32),
    Address(i32),
    Label(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum UnresolvedInstruction<'a> {
    Nop,
    Int,
    Mov(Target, UnresolvedSource<'a>),
    Push(UnresolvedSource<'a>),
    Pop(Target),
    Pushf,
    Popf,
    Inc(Target),
    Dec(Target),
    Add(Target, UnresolvedSource<'a>),
    Sub(Target, UnresolvedSource<'a>),
    Mul(Target, UnresolvedSource<'a>),
    Div(Target, UnresolvedSource<'a>),
    Mod(UnresolvedSource<'a>, UnresolvedSource<'a>),
    Rem(Target),
    Not(Target),
    Xor(Target, UnresolvedSource<'a>),
    Or(Target, UnresolvedSource<'a>),
    And(Target, UnresolvedSource<'a>),
    Shl(Target, UnresolvedSource<'a>),
    Shr(Target, UnresolvedSource<'a>),
    Cmp(UnresolvedSource<'a>, UnresolvedSource<'a>),
    Jmp(UnresolvedSource<'a>),
    Call(UnresolvedSource<'a>),
    Ret,
    Je(UnresolvedSource<'a>),
    Jne(UnresolvedSource<'a>),
    Jg(UnresolvedSource<'a>),
    Jge(UnresolvedSource<'a>),
    Jl(UnresolvedSource<'a>),
    Jle(UnresolvedSource<'a>),
    Prn(UnresolvedSource<'a>),
}

fn parse_value(value: &str) -> Result<i32, ParseIntError> {
    if value.ends_with("|h") {
        i32::from_str_radix(&value[0..value.len() - 2], 16)
    } else if value.ends_with("h") {
        i32::from_str_radix(&value[0..value.len() - 1], 16)
    } else if value.ends_with("|b") {
        i32::from_str_radix(&value[0..value.len() - 2], 2)
    } else if value.ends_with("b") {
        i32::from_str_radix(&value[0..value.len() - 1], 2)
    } else if value.starts_with("0x") {
        i32::from_str_radix(&value[2..], 16)
    } else if value.starts_with("-0x") {
        i32::from_str_radix(&value[3..], 16).map(|i| -i)
    } else {
        i32::from_str_radix(value, 10)
    }
}

fn parse_target(tokens: &[&str], index: usize) -> Result<Target, ParseErrorKind> {
    match parse_source(tokens, index) {
        Ok(UnresolvedSource::Register(reg)) => Ok(Target::Register(reg)),
        Ok(UnresolvedSource::Address(addr)) => Ok(Target::Address(addr)),
        Ok(_) => Err(ParseErrorKind::InvalidOperand(tokens[index].to_owned())),
        Err(e) => Err(e),
    }
}

fn parse_source<'a>(
    tokens: &[&'a str],
    index: usize,
) -> Result<UnresolvedSource<'a>, ParseErrorKind> {
    let token = tokens
        .get(index)
        .ok_or(ParseErrorKind::MissingOperand(index))?;

    if let Some(reg) = parse_register(token) {
        Ok(UnresolvedSource::Register(reg))
    } else if token.starts_with('[') && token.ends_with(']') {
        match parse_value(token[1..token.len() - 1].trim()) {
            Ok(value) => {
                if value >= 0 {
                    Ok(UnresolvedSource::Address(value))
                } else {
                    Err(ParseErrorKind::InvalidOperand((*token).to_owned()))
                }
            }
            Err(_) => Err(ParseErrorKind::InvalidOperand((*token).to_owned())),
        }
    } else {
        match parse_value(token) {
            Ok(value) => Ok(UnresolvedSource::Value(value)),
            Err(_) => {
                if is_valid_label(token) {
                    Ok(UnresolvedSource::Label(token))
                } else {
                    Err(ParseErrorKind::InvalidOperand((*token).to_owned()))
                }
            }
        }
    }
}

impl UnresolvedInstruction<'_> {
    pub(super) fn parse<'a>(
        tokens: &[&'a str],
    ) -> Result<UnresolvedInstruction<'a>, ParseErrorKind> {
        let mnemonic = tokens[0];
        let mut arg_number = 0;

        macro_rules! target {
            () => {{
                arg_number += 1;
                parse_target(tokens, arg_number)?
            }};
        }

        macro_rules! source {
            () => {{
                arg_number += 1;
                parse_source(tokens, arg_number)?
            }};
        }

        macro_rules! instr {
            ($mnemonic:literal, $instruction:ident) => {
                if (mnemonic == $mnemonic) {
                    return if tokens.len() == 1 {
                        Ok(UnresolvedInstruction::$instruction)
                    } else {
                        Err(ParseErrorKind::ExtraToken(tokens[1].to_owned()))
                    }
                }
            };
            ($mnemonic:literal, $instruction:ident, $($arg:tt),+) => {
                if (mnemonic == $mnemonic) {
                    let result = UnresolvedInstruction::$instruction($($arg!()),+);
                    return if arg_number == tokens.len() - 1 {
                        Ok(result)
                    } else {
                        Err(ParseErrorKind::ExtraToken(tokens[arg_number + 1].to_owned()))
                    }
                }
            };
        }

        instr!("nop", Nop);
        instr!("int", Int);
        instr!("mov", Mov, target, source);
        instr!("push", Push, source);
        instr!("pop", Pop, target);
        instr!("pushf", Pushf);
        instr!("popf", Popf);
        instr!("inc", Inc, target);
        instr!("dec", Dec, target);
        instr!("add", Add, target, source);
        instr!("sub", Sub, target, source);
        instr!("mul", Mul, target, source);
        instr!("div", Div, target, source);
        instr!("mod", Mod, source, source);
        instr!("rem", Rem, target);
        instr!("not", Not, target);
        instr!("xor", Xor, target, source);
        instr!("or", Or, target, source);
        instr!("and", And, target, source);
        instr!("shl", Shl, target, source);
        instr!("shr", Shr, target, source);
        instr!("cmp", Cmp, source, source);
        instr!("jmp", Jmp, source);
        instr!("call", Call, source);
        instr!("ret", Ret);
        instr!("je", Je, source);
        instr!("jne", Jne, source);
        instr!("jg", Jg, source);
        instr!("jge", Jge, source);
        instr!("jl", Jl, source);
        instr!("jle", Jle, source);
        instr!("prn", Prn, source);

        return Err(ParseErrorKind::InvalidInstruction(tokens[0].to_owned()));
    }
}

#[cfg(test)]
mod tests {
    use super::{UnresolvedInstruction::*, *};

    fn run(source: &str, expected: UnresolvedInstruction) {
        let tokens: Vec<_> = source.split(" ").collect();
        assert_eq!(UnresolvedInstruction::parse(&tokens).unwrap(), expected);
    }

    fn run_error(source: &str, expected: ParseErrorKind) {
        let tokens: Vec<_> = source.split(" ").collect();
        assert_eq!(
            UnresolvedInstruction::parse(&tokens).err().unwrap(),
            expected
        );
    }

    #[test]
    fn can_parse_all_instructions() {
        let eax = Target::Register(Register::Eax);
        let ebx = UnresolvedSource::Register(Register::Ebx);
        let ecx = UnresolvedSource::Register(Register::Ecx);

        run("nop", Nop);
        run("int", Int);
        run("mov eax ebx", Mov(eax, ebx));
        run("push ebx", Push(ebx));
        run("pop eax", Pop(eax));
        run("pushf", Pushf);
        run("popf", Popf);
        run("inc eax", Inc(eax));
        run("dec eax", Dec(eax));
        run("add eax ebx", Add(eax, ebx));
        run("sub eax ebx", Sub(eax, ebx));
        run("mul eax ebx", Mul(eax, ebx));
        run("div eax ebx", Div(eax, ebx));
        run("mod ebx ecx", Mod(ebx, ecx));
        run("rem eax", Rem(eax));
        run("not eax", Not(eax));
        run("xor eax ebx", Xor(eax, ebx));
        run("or eax ebx", Or(eax, ebx));
        run("and eax ebx", And(eax, ebx));
        run("shl eax ebx", Shl(eax, ebx));
        run("shr eax ebx", Shr(eax, ebx));
        run("cmp ebx ecx", Cmp(ebx, ecx));
        run("jmp ebx", Jmp(ebx));
        run("call ebx", Call(ebx));
        run("ret", Ret);
        run("je ebx", Je(ebx));
        run("jne ebx", Jne(ebx));
        run("jg ebx", Jg(ebx));
        run("jge ebx", Jge(ebx));
        run("jl ebx", Jl(ebx));
        run("jle ebx", Jle(ebx));
        run("prn ebx", Prn(ebx));
    }

    #[test]
    fn can_use_register_as_target() {
        run("pop eax", Pop(Target::Register(Register::Eax)));
        run("pop ebx", Pop(Target::Register(Register::Ebx)));
        run("pop ecx", Pop(Target::Register(Register::Ecx)));
        run("pop edx", Pop(Target::Register(Register::Edx)));
        run("pop esi", Pop(Target::Register(Register::Esi)));
        run("pop edi", Pop(Target::Register(Register::Edi)));
        run("pop esp", Pop(Target::Register(Register::Esp)));
        run("pop ebp", Pop(Target::Register(Register::Ebp)));
        run("pop eip", Pop(Target::Register(Register::Eip)));
        run("pop r08", Pop(Target::Register(Register::R08)));
        run("pop r09", Pop(Target::Register(Register::R09)));
        run("pop r10", Pop(Target::Register(Register::R10)));
        run("pop r11", Pop(Target::Register(Register::R11)));
        run("pop r12", Pop(Target::Register(Register::R12)));
        run("pop r13", Pop(Target::Register(Register::R13)));
        run("pop r14", Pop(Target::Register(Register::R14)));
        run("pop r15", Pop(Target::Register(Register::R15)));
    }

    #[test]
    fn can_use_memory_as_target() {
        run("pop [123]", Pop(Target::Address(123)));
        run("pop [12ah]", Pop(Target::Address(0x12a)));
        run("pop [12a|h]", Pop(Target::Address(0x12a)));
        run("pop [0x12a]", Pop(Target::Address(0x12a)));
        run("pop [01001010111b]", Pop(Target::Address(0b01001010111)));
        run("pop [01001010111|b]", Pop(Target::Address(0b01001010111)));
    }

    #[test]
    fn cannot_use_literal_as_target() {
        run_error("pop 1", ParseErrorKind::InvalidOperand("1".to_owned()));
    }

    #[test]
    fn cannot_use_label_as_target() {
        run_error(
            "pop label",
            ParseErrorKind::InvalidOperand("label".to_owned()),
        );
    }

    #[test]
    fn cannot_use_random_garbage_as_target() {
        run_error(
            "pop 23()C",
            ParseErrorKind::InvalidOperand("23()C".to_owned()),
        );
    }

    #[test]
    fn can_use_register_as_source() {
        run("push eax", Push(UnresolvedSource::Register(Register::Eax)));
        run("push ebx", Push(UnresolvedSource::Register(Register::Ebx)));
        run("push ecx", Push(UnresolvedSource::Register(Register::Ecx)));
        run("push edx", Push(UnresolvedSource::Register(Register::Edx)));
        run("push esi", Push(UnresolvedSource::Register(Register::Esi)));
        run("push edi", Push(UnresolvedSource::Register(Register::Edi)));
        run("push esp", Push(UnresolvedSource::Register(Register::Esp)));
        run("push ebp", Push(UnresolvedSource::Register(Register::Ebp)));
        run("push eip", Push(UnresolvedSource::Register(Register::Eip)));
        run("push r08", Push(UnresolvedSource::Register(Register::R08)));
        run("push r09", Push(UnresolvedSource::Register(Register::R09)));
        run("push r10", Push(UnresolvedSource::Register(Register::R10)));
        run("push r11", Push(UnresolvedSource::Register(Register::R11)));
        run("push r12", Push(UnresolvedSource::Register(Register::R12)));
        run("push r13", Push(UnresolvedSource::Register(Register::R13)));
        run("push r14", Push(UnresolvedSource::Register(Register::R14)));
        run("push r15", Push(UnresolvedSource::Register(Register::R15)));
    }

    #[test]
    fn can_use_memory_as_source() {
        run("push [123]", Push(UnresolvedSource::Address(123)));
        run("push [12ah]", Push(UnresolvedSource::Address(0x12a)));
        run("push [12a|h]", Push(UnresolvedSource::Address(0x12a)));
        run("push [0x12a]", Push(UnresolvedSource::Address(0x12a)));
        run(
            "push [01001010111b]",
            Push(UnresolvedSource::Address(0b01001010111)),
        );
        run(
            "push [01001010111|b]",
            Push(UnresolvedSource::Address(0b01001010111)),
        );
    }

    #[test]
    fn can_use_literal_as_source() {
        run("push 123", Push(UnresolvedSource::Value(123)));
        run("push 12ah", Push(UnresolvedSource::Value(0x12a)));
        run("push 12a|h", Push(UnresolvedSource::Value(0x12a)));
        run("push 0x12a", Push(UnresolvedSource::Value(0x12a)));
        run(
            "push 01001010111b",
            Push(UnresolvedSource::Value(0b01001010111)),
        );
        run(
            "push 01001010111|b",
            Push(UnresolvedSource::Value(0b01001010111)),
        );

        run("push -123", Push(UnresolvedSource::Value(-123)));
        run("push -12ah", Push(UnresolvedSource::Value(-0x12a)));
        run("push -12a|h", Push(UnresolvedSource::Value(-0x12a)));
        run("push -0x12a", Push(UnresolvedSource::Value(-0x12a)));
        run(
            "push -01001010111b",
            Push(UnresolvedSource::Value(-0b01001010111)),
        );
        run(
            "push -01001010111|b",
            Push(UnresolvedSource::Value(-0b01001010111)),
        );
    }

    #[test]
    fn can_use_label_as_source() {
        run("push label", Push(UnresolvedSource::Label("label")));
    }

    #[test]
    fn cannot_use_random_garbage_as_source() {
        run_error(
            "push 23()C",
            ParseErrorKind::InvalidOperand("23()C".to_owned()),
        );
    }
}
