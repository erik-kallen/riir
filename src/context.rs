use crate::{
    htab::HashTable,
    instruction::{Instruction, Register, Source, Target, NUM_REGISTERS},
    lexer::LexerContext,
    parser::{parse, ParseError},
    preprocessor::{preprocess, PreprocessingError},
};

const MEMORY_SIZE: usize = 16 * 1024 * 1024; // 64 MB (8M i32)
const STACK_SIZE: usize = 512 * 1024; // 2 MB (512k i32)

pub struct Program {
    pub instructions: Vec<Instruction>,
    pub start_instruction_index: i32,
}

pub struct Memory {
    pub flags: i32,
    pub remainder: i32,
    pub mem_space: Vec<i32>,
    pub registers: [i32; NUM_REGISTERS],
}

#[derive(Debug)]
pub enum LoadError {
    PreprocessingError(PreprocessingError),
    ParseError(ParseError),
}

#[derive(Debug, PartialEq)]
pub enum ExecutionError {
    InstructionOutOfRange(i32),
    DataAddressOutOfRange(i32),
}

impl From<PreprocessingError> for LoadError {
    fn from(error: PreprocessingError) -> LoadError {
        LoadError::PreprocessingError(error)
    }
}

impl From<ParseError> for LoadError {
    fn from(error: ParseError) -> LoadError {
        LoadError::ParseError(error)
    }
}

impl Program {
    pub fn load(source: String) -> Result<Program, LoadError> {
        let mut defines = HashTable::default();
        let source = preprocess(source, &mut defines)?;

        let lexer = LexerContext::lex(&source, &defines);

        let program = parse(&lexer.tokens())?;

        Ok(program)
    }

    pub fn run(self: &Program) -> Result<(), ExecutionError> {
        let mut memory = self.initialize();

        loop {
            if !self.step(&mut memory)? {
                break;
            }
        }

        Ok(())
    }

    pub fn initialize(self: &Program) -> Memory {
        let mut memory = Memory::new(MEMORY_SIZE, STACK_SIZE);
        memory.registers[Register::Eip as usize] = self.start_instruction_index;
        memory
    }

    pub fn step(self: &Program, memory: &mut Memory) -> Result<bool, ExecutionError> {
        let instruction_index = memory.registers[Register::Eip as usize];

        if instruction_index < 0 || instruction_index > self.instructions.len() as i32 {
            return Err(ExecutionError::InstructionOutOfRange(instruction_index));
        } else if instruction_index == self.instructions.len() as i32 {
            return Ok(false);
        }

        macro_rules! read {
            ($source:ident) => {
                match $source {
                    Source::Register(reg) => memory.registers[reg as usize],
                    Source::Value(value) => value,
                    Source::Address(addr) => {
                        if addr < 0 || addr as usize >= memory.mem_space.len() {
                            return Err(ExecutionError::DataAddressOutOfRange(addr));
                        }
                        memory.mem_space[addr as usize]
                    }
                };
            };
        }

        macro_rules! readt {
            ($target:ident) => {
                match $target {
                    Target::Register(reg) => memory.registers[reg as usize],
                    Target::Address(addr) => {
                        if addr < 0 || addr as usize >= memory.mem_space.len() {
                            return Err(ExecutionError::DataAddressOutOfRange(addr));
                        }
                        memory.mem_space[addr as usize]
                    }
                }
            };
        }

        macro_rules! write {
            ($target:ident, $value:expr) => {
                match $target {
                    Target::Register(reg) => memory.registers[reg as usize] = $value,
                    Target::Address(addr) => {
                        if addr < 0 || addr as usize >= memory.mem_space.len() {
                            return Err(ExecutionError::DataAddressOutOfRange(addr));
                        }
                        memory.mem_space[addr as usize] = $value;
                    }
                };
            };
        }

        let mut should_advance = true;
        macro_rules! jump {
            ($source:ident) => {
                memory.registers[Register::Eip as usize] = read!($source);
                should_advance = false;
            };
            ($condition:expr, $source:ident) => {
                if $condition {
                    jump!($source);
                }
            };
        }

        macro_rules! push {
            ($value:expr) => {
                let addr = memory.registers[Register::Esp as usize] - 1;
                if addr < 0 || addr as usize >= memory.mem_space.len() {
                    return Err(ExecutionError::DataAddressOutOfRange(addr));
                }
                memory.mem_space[addr as usize] = $value;
                memory.registers[Register::Esp as usize] = addr;
            };
        }

        macro_rules! pop {
            () => {{
                let addr = memory.registers[Register::Esp as usize];
                if addr < 0 || addr as usize >= memory.mem_space.len() {
                    return Err(ExecutionError::DataAddressOutOfRange(addr));
                }
                memory.registers[Register::Esp as usize] = addr + 1;
                memory.mem_space[addr as usize]
            }};
        }

        match self.instructions[instruction_index as usize] {
            Instruction::Nop => {}
            Instruction::Int => { /* unimplemented */ }
            Instruction::Mov(target, source) => {
                write!(target, read!(source));
            }
            Instruction::Push(source) => {
                push!(read!(source));
            }
            Instruction::Pop(target) => {
                write!(target, pop!());
            }
            Instruction::Pushf => {
                push!(memory.flags);
            }
            Instruction::Popf => {
                memory.flags = pop!();
            }
            Instruction::Inc(target) => {
                write!(target, readt!(target) + 1);
            }
            Instruction::Dec(target) => {
                write!(target, readt!(target) - 1);
            }
            Instruction::Add(target, source) => {
                write!(target, readt!(target) + read!(source));
            }
            Instruction::Sub(target, source) => {
                write!(target, readt!(target) - read!(source));
            }
            Instruction::Mul(target, source) => {
                write!(target, readt!(target) * read!(source));
            }
            Instruction::Div(target, source) => {
                write!(target, readt!(target) / read!(source));
            }
            Instruction::Mod(source1, source2) => {
                memory.remainder = read!(source1) % read!(source2);
            }
            Instruction::Rem(target) => {
                write!(target, memory.remainder);
            }
            Instruction::Not(target) => {
                write!(target, !readt!(target));
            }
            Instruction::Xor(target, source) => {
                write!(target, readt!(target) ^ read!(source));
            }
            Instruction::Or(target, source) => {
                write!(target, readt!(target) | read!(source));
            }
            Instruction::And(target, source) => {
                write!(target, readt!(target) & read!(source));
            }
            Instruction::Shl(target, source) => {
                write!(target, readt!(target) << read!(source));
            }
            Instruction::Shr(target, source) => {
                write!(target, readt!(target) >> read!(source));
            }
            Instruction::Cmp(source1, source2) => {
                let value1 = read!(source1);
                let value2 = read!(source2);

                memory.flags =
                    if value1 == value2 { 1 } else { 0 } | if value1 > value2 { 2 } else { 0 }
            }
            Instruction::Jmp(source) => {
                jump!(source);
            }
            Instruction::Call(source) => {
                push!(instruction_index + 1);
                jump!(source);
            }
            Instruction::Ret => {
                memory.registers[Register::Eip as usize] = pop!();
                should_advance = false;
            }
            Instruction::Je(source) => {
                jump!(memory.flags & 0x1 != 0, source);
            }
            Instruction::Jne(source) => {
                jump!(memory.flags & 0x1 == 0, source);
            }
            Instruction::Jg(source) => {
                jump!(memory.flags & 0x2 != 0, source);
            }
            Instruction::Jge(source) => {
                jump!(memory.flags & 0x3 != 0, source);
            }
            Instruction::Jl(source) => {
                jump!(memory.flags & 0x3 == 0, source);
            }
            Instruction::Jle(source) => {
                jump!(memory.flags & 0x2 == 0, source);
            }
            Instruction::Prn(source) => println!("{}", read!(source)),
        };

        if should_advance {
            memory.registers[Register::Eip as usize] = instruction_index + 1;
        }

        Ok(true)
    }
}

impl Memory {
    pub fn new(size: usize, stack_size: usize) -> Memory {
        let mut memory = Memory {
            flags: 0,
            remainder: 0,
            mem_space: vec![0; size],
            registers: [0; NUM_REGISTERS],
        };

        memory.registers[Register::Esp as usize] = stack_size as i32;
        memory.registers[Register::Ebp as usize] = stack_size as i32;

        memory
    }
}
