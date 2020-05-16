use crate::{
    htab::HashTable,
    instruction::{Instruction, Source, Target},
    lexer::{lexer_create, tvm_lex, tvm_lexer_destroy, LexerContext},
    memory::Memory,
    parser::old_interface::{tvm_parse_labels, tvm_parse_program},
    preprocessor::{preprocess, PreprocessingError},
    program::Program,
};
use std::{
    ffi::{CStr, CString},
    fs,
    os::raw::{c_char, c_int, c_void},
    pin::Pin,
};

const MEMORY_SIZE: usize = 64 * 1024 * 1024; // 64 MB
const STACK_SIZE: usize = 2 * 1024 * 1024; // 2 MB

#[repr(C)]
pub struct Context {
    pub prog_ptr: *mut c_void,
    pub program: Pin<Box<Program>>,
    pub memory: Memory,
}

pub enum InterpretingError {
    PreprocessingError(PreprocessingError),
    ParseLabelsError,
    ParseProgramError,
}

impl From<PreprocessingError> for InterpretingError {
    fn from(error: PreprocessingError) -> InterpretingError {
        InterpretingError::PreprocessingError(error)
    }
}

impl Context {
    pub fn new() -> Context {
        let mut program = Program::new();
        let mut memory = Memory::new(MEMORY_SIZE);
        memory.create_stack(STACK_SIZE);

        let context = Context {
            prog_ptr: unsafe { Pin::get_unchecked_mut(program.as_mut()) as *mut Program as *mut _ },
            program,
            memory,
        };

        context
    }

    pub fn interpret(self: &mut Context, source: String) -> Result<(), InterpretingError> {
        unsafe {
            let defines = &mut *(self.program.defines_ptr as *mut HashTable);
            let source = preprocess(source, defines)?;

            let source = CString::new(source).unwrap();
            let lexer = lexer_create();
            let source_mut = source.into_raw();
            tvm_lex(lexer, source_mut, self.program.defines_ptr);
            let _ = CString::from_raw(source_mut);

            let tokens_ptr = (*(lexer as *mut LexerContext)).tokens_ptr;

            if tvm_parse_labels((self as *mut Context).cast(), tokens_ptr) != 0 {
                return Err(InterpretingError::ParseLabelsError);
            }
            if tvm_parse_program((self as *mut Context).cast(), tokens_ptr) != 0 {
                return Err(InterpretingError::ParseProgramError);
            }

            tvm_lexer_destroy(lexer);
        }

        Ok(())
    }

    pub fn run(self: &mut Context) {
        self.memory
            .set_current_instruction_index(self.program.start_instruction_index);

        loop {
            let current_instruction_index = self.memory.get_current_instruction_index();

            if current_instruction_index == self.program.instructions.len() as i32 {
                break;
            }

            self.step();
        }
    }

    pub fn step(self: &mut Context) {
        let instruction_index = self.memory.get_current_instruction_index();

        if instruction_index < 0 || instruction_index >= self.program.instructions.len() as i32 {
            panic!("Tried to read instruction outside the program area");
        }

        macro_rules! read {
            ($source:ident) => {
                match $source {
                    Source::Register(reg) => unsafe { self.memory.registers[reg as usize].value },
                    Source::Value(value) => value,
                    Source::Address(addr) => self.memory.mem_space[addr],
                };
            };
        }

        macro_rules! readt {
            ($target:ident) => {
                match $target {
                    Target::Register(reg) => unsafe { self.memory.registers[reg as usize].value },
                    Target::Address(addr) => self.memory.mem_space[addr],
                }
            };
        }

        macro_rules! write {
            ($target:ident, $value:expr) => {
                match $target {
                    Target::Register(reg) => self.memory.registers[reg as usize].value = $value,
                    Target::Address(addr) => self.memory.mem_space[addr] = $value,
                };
            };
        }

        macro_rules! jump {
            ($source:ident) => {
                self.memory
                    .set_current_instruction_index(read!($source) - 1);
            };
            ($condition:expr, $source:ident) => {
                if $condition {
                    self.memory
                        .set_current_instruction_index(read!($source) - 1);
                }
            };
        }

        match self.program.instructions[instruction_index as usize] {
            Instruction::Nop => {}
            Instruction::Int => { /* unimplemented */ }
            Instruction::Mov(target, source) => {
                write!(target, read!(source));
            }
            Instruction::Push(source) => {
                let value = read!(source);
                unsafe { self.memory.push_stack(value) };
            }
            Instruction::Pop(target) => {
                write!(target, unsafe { self.memory.pop_stack() });
            }
            Instruction::Pushf => {
                unsafe { self.memory.push_stack(self.memory.flags) };
            }
            Instruction::Popf => {
                unsafe { self.memory.flags = self.memory.pop_stack() };
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
                self.memory.remainder = read!(source1) % read!(source2);
            }
            Instruction::Rem(target) => {
                write!(target, self.memory.remainder);
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

                self.memory.flags =
                    if value1 == value2 { 1 } else { 0 } | if value1 > value2 { 2 } else { 0 }
            }
            Instruction::Jmp(source) => {
                jump!(source);
            }
            Instruction::Call(source) => {
                unsafe { self.memory.push_stack(instruction_index as i32) };
                jump!(source);
            }
            Instruction::Ret => {
                let target = unsafe { self.memory.pop_stack() };
                self.memory.set_current_instruction_index(target);
            }
            Instruction::Je(source) => {
                jump!(self.memory.flags & 0x1 != 0, source);
            }
            Instruction::Jne(source) => {
                jump!(self.memory.flags & 0x1 == 0, source);
            }
            Instruction::Jg(source) => {
                jump!(self.memory.flags & 0x2 != 0, source);
            }
            Instruction::Jge(source) => {
                jump!(self.memory.flags & 0x3 != 0, source);
            }
            Instruction::Jl(source) => {
                jump!(self.memory.flags & 0x3 == 0, source);
            }
            Instruction::Jle(source) => {
                jump!(self.memory.flags & 0x2 == 0, source);
            }
            Instruction::Prn(source) => println!("{}", read!(source)),
        };

        self.memory
            .set_current_instruction_index(self.memory.get_current_instruction_index() + 1);
    }
}

fn read_to_string_with_possible_extension(
    filename: &str,
    extension: &str,
) -> Result<String, std::io::Error> {
    match fs::read_to_string(filename) {
        Ok(s) => return Ok(s),
        Err(error) => match error.kind() {
            std::io::ErrorKind::NotFound => (),
            _ => return Err(error),
        },
    };

    fs::read_to_string(filename.to_owned() + extension)
}

#[no_mangle]
pub unsafe extern "C" fn tvm_vm_create() -> *mut c_void {
    let context = Box::new(Context::new());
    Box::into_raw(context) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn tvm_vm_interpret(vm: *mut c_void, filename: *const c_char) -> c_int {
    let vm = &mut *(vm as *mut Context);
    let filename = match CStr::from_ptr(filename).to_str() {
        Ok(f) => f,
        Err(_) => return 1,
    };

    let source = match read_to_string_with_possible_extension(filename, ".vm") {
        Ok(s) => s,
        Err(_) => return 1,
    };

    match vm.interpret(source) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn tvm_vm_run(vm: *mut c_void) {
    let vm = &mut *(vm as *mut Context);

    vm.run();
}

#[no_mangle]
pub unsafe extern "C" fn tvm_step(vm: *mut c_void, _instr_idx: *mut c_int) {
    let vm = &mut *(vm as *mut Context);

    vm.step();
}

#[no_mangle]
pub unsafe extern "C" fn tvm_vm_destroy(vm: *mut c_void) {
    if vm.is_null() {
        return;
    }

    let vm = Box::from_raw(vm as *mut Context);

    drop(vm);
}
