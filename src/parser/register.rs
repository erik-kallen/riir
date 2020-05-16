use crate::instruction::Register;
use std::collections::HashMap;

lazy_static! {
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

pub(super) fn parse_register(name: &str) -> Option<Register> {
    REGISTER_MAP.get(name).map(|v| *v)
}
