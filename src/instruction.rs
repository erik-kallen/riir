use std::{collections::HashMap, num::ParseIntError};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Register {
    Eax = 0,
    Ebx = 1,
    Ecx = 2,
    Edx = 3,
    Esi = 4,
    Edi = 5,
    Esp = 6,
    Ebp = 7,
    Eip = 8,
    R08 = 9,
    R09 = 10,
    R10 = 11,
    R11 = 12,
    R12 = 13,
    R13 = 14,
    R14 = 15,
    R15 = 16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operand {
    Register(Register),
    Value(i32),
    Address(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive)]
pub enum OpCode {
    Nop = 0x00,
    Int = 0x01,
    Mov = 0x02,
    Push = 0x03,
    Pop = 0x04,
    Pushf = 0x05,
    Popf = 0x06,
    Inc = 0x07,
    Dec = 0x08,
    Add = 0x09,
    Sub = 0x0A,
    Mul = 0x0B,
    Div = 0x0C,
    Mod = 0x0D,
    Rem = 0x0E,
    Not = 0x0F,
    Xor = 0x10,
    Or = 0x11,
    And = 0x12,
    Shl = 0x13,
    Shr = 0x14,
    Cmp = 0x15,
    Jmp = 0x16,
    Call = 0x17,
    Ret = 0x18,
    Je = 0x19,
    Jne = 0x1A,
    Jg = 0x1B,
    Jge = 0x1C,
    Jl = 0x1D,
    Jle = 0x1E,
    Prn = 0x1F,
}

lazy_static! {
    static ref OPCODE_MAP: HashMap<&'static str, OpCode> = vec!(
        ("nop", OpCode::Nop),
        ("int", OpCode::Int),
        ("mov", OpCode::Mov),
        ("push", OpCode::Push),
        ("pop", OpCode::Pop),
        ("pushf", OpCode::Pushf),
        ("popf", OpCode::Popf),
        ("inc", OpCode::Inc),
        ("dec", OpCode::Dec),
        ("add", OpCode::Add),
        ("sub", OpCode::Sub),
        ("mul", OpCode::Mul),
        ("div", OpCode::Div),
        ("mod", OpCode::Mod),
        ("rem", OpCode::Rem),
        ("not", OpCode::Not),
        ("xor", OpCode::Xor),
        ("or", OpCode::Or),
        ("and", OpCode::And),
        ("shl", OpCode::Shl),
        ("shr", OpCode::Shr),
        ("cmp", OpCode::Cmp),
        ("jmp", OpCode::Jmp),
        ("call", OpCode::Call),
        ("ret", OpCode::Ret),
        ("je", OpCode::Je),
        ("jne", OpCode::Jne),
        ("jg", OpCode::Jg),
        ("jge", OpCode::Jge),
        ("jl", OpCode::Jl),
        ("jle", OpCode::Jle),
        ("prn", OpCode::Prn),
    )
    .into_iter()
    .collect();
    static ref REGISTER_MAP: HashMap<&'static str, Register> = vec!(
        ("eax", Register::Eax),
        ("ebx", Register::Ebx),
        ("ecx", Register::Ecx),
        ("edx", Register::Edx),
        ("esi", Register::Esi),
        ("edi", Register::Edi),
        ("esp", Register::Esp),
        ("ebp", Register::Ebp),
        ("eip", Register::Eip),
        ("r08", Register::R08),
        ("r09", Register::R09),
        ("r10", Register::R10),
        ("r11", Register::R11),
        ("r12", Register::R12),
        ("r13", Register::R13),
        ("r14", Register::R14),
        ("r15", Register::R15),
    )
    .into_iter()
    .collect();
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

impl OpCode {
    pub fn parse(mnemonic: &str) -> Option<OpCode> {
        OPCODE_MAP.get(mnemonic).map(|v| *v)
    }
}

impl Register {
    pub fn parse(name: &str) -> Option<Register> {
        REGISTER_MAP.get(name).map(|v| *v)
    }
}

impl Operand {
    pub fn parse(token: &str) -> Option<Operand> {
        if let Some(reg) = REGISTER_MAP.get(token) {
            Some(Operand::Register(*reg))
        } else if token.starts_with('[') && token.ends_with(']') {
            match parse_value(token[1..token.len() - 1].trim()) {
                Ok(value) => {
                    if value >= 0 {
                        Some(Operand::Address(value as usize))
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        } else {
            match parse_value(token) {
                Ok(value) => Some(Operand::Value(value)),
                Err(_) => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    mod operand {
        use super::super::{Operand::*, Register::*, *};

        #[test]
        fn can_parse_value() {
            assert_eq!(Operand::parse("123"), Some(Value(123)));
            assert_eq!(Operand::parse("12ah"), Some(Value(0x12a)));
            assert_eq!(Operand::parse("12a|h"), Some(Value(0x12a)));
            assert_eq!(Operand::parse("0x12a"), Some(Value(0x12a)));
            assert_eq!(Operand::parse("01001010111b"), Some(Value(0b01001010111)));
            assert_eq!(Operand::parse("01001010111|b"), Some(Value(0b01001010111)));

            assert_eq!(Operand::parse("-123"), Some(Value(-123)));
            assert_eq!(Operand::parse("-12ah"), Some(Value(-0x12a)));
            assert_eq!(Operand::parse("-12a|h"), Some(Value(-0x12a)));
            assert_eq!(Operand::parse("-0x12a"), Some(Value(-0x12a)));
            assert_eq!(Operand::parse("-01001010111b"), Some(Value(-0b01001010111)));
            assert_eq!(
                Operand::parse("-01001010111|b"),
                Some(Value(-0b01001010111))
            );
        }

        #[test]
        fn can_parse_address() {
            assert_eq!(Operand::parse("[123]"), Some(Address(123)));
            assert_eq!(Operand::parse("[12ah]"), Some(Address(0x12a)));
            assert_eq!(Operand::parse("[12a|h]"), Some(Address(0x12a)));
            assert_eq!(Operand::parse("[0x12a]"), Some(Address(0x12a)));
            assert_eq!(
                Operand::parse("[01001010111b]"),
                Some(Address(0b01001010111))
            );
            assert_eq!(
                Operand::parse("[01001010111|b]"),
                Some(Address(0b01001010111))
            );
        }

        #[test]
        fn can_parse_register() {
            assert_eq!(Operand::parse("eax").unwrap(), Register(Eax));
            assert_eq!(Operand::parse("ebx").unwrap(), Register(Ebx));
            assert_eq!(Operand::parse("ecx").unwrap(), Register(Ecx));
            assert_eq!(Operand::parse("edx").unwrap(), Register(Edx));
            assert_eq!(Operand::parse("esi").unwrap(), Register(Esi));
            assert_eq!(Operand::parse("edi").unwrap(), Register(Edi));
            assert_eq!(Operand::parse("esp").unwrap(), Register(Esp));
            assert_eq!(Operand::parse("ebp").unwrap(), Register(Ebp));
            assert_eq!(Operand::parse("eip").unwrap(), Register(Eip));
            assert_eq!(Operand::parse("r08").unwrap(), Register(R08));
            assert_eq!(Operand::parse("r09").unwrap(), Register(R09));
            assert_eq!(Operand::parse("r10").unwrap(), Register(R10));
            assert_eq!(Operand::parse("r11").unwrap(), Register(R11));
            assert_eq!(Operand::parse("r12").unwrap(), Register(R12));
            assert_eq!(Operand::parse("r13").unwrap(), Register(R13));
            assert_eq!(Operand::parse("r14").unwrap(), Register(R14));
            assert_eq!(Operand::parse("r15").unwrap(), Register(R15));
        }

        #[test]
        fn parse_returns_none_for_invalid_operands() {
            assert!(Operand::parse("invalid").is_none());
            assert!(Operand::parse("[invalid]").is_none());
            assert!(Operand::parse("[-1]").is_none());
        }
    }
}
