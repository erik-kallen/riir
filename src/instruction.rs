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

pub const NUM_REGISTERS: usize = 17;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Source {
    Register(Register),
    Value(i32),
    Address(i32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    Register(Register),
    Address(i32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Instruction {
    Nop,
    Int,
    Mov(Target, Source),
    Push(Source),
    Pop(Target),
    Pushf,
    Popf,
    Inc(Target),
    Dec(Target),
    Add(Target, Source),
    Sub(Target, Source),
    Mul(Target, Source),
    Div(Target, Source),
    Mod(Source, Source),
    Rem(Target),
    Not(Target),
    Xor(Target, Source),
    Or(Target, Source),
    And(Target, Source),
    Shl(Target, Source),
    Shr(Target, Source),
    Cmp(Source, Source),
    Jmp(Source),
    Call(Source),
    Ret,
    Je(Source),
    Jne(Source),
    Jg(Source),
    Jge(Source),
    Jl(Source),
    Jle(Source),
    Prn(Source),
}
