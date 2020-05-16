use crate::{
    ffi,
    htab::HashTable,
    instruction::OpCode::*,
    memory::Memory,
    preprocessor::{preprocess, PreprocessingError},
    program::Program,
};
use num_traits::FromPrimitive;
use std::{
    ffi::{CStr, CString},
    fs,
    os::raw::{c_char, c_int},
    pin::Pin,
};

const MEMORY_SIZE: usize = ffi::MIN_MEMORY_SIZE as usize;
const STACK_SIZE: usize = ffi::MIN_STACK_SIZE as usize;

#[repr(C)]
pub struct Context {
    pub prog_ptr: *mut ffi::tvm_prog,
    pub mem_ptr: *mut ffi::tvm_mem,
    pub program: Pin<Box<Program>>,
    pub memory: Pin<Box<Memory>>,
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
        unsafe {
            Pin::get_unchecked_mut(memory.as_mut()).create_stack(STACK_SIZE);
        }

        let context = Context {
            prog_ptr: unsafe {
                Pin::get_unchecked_mut(program.as_mut()) as *mut Program as *mut ffi::tvm_prog
            },
            mem_ptr: unsafe {
                Pin::get_unchecked_mut(memory.as_mut()) as *mut Memory as *mut ffi::tvm_mem
            },
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
            let lexer = ffi::lexer_create();
            let source_mut = source.into_raw();
            ffi::tvm_lex(lexer, source_mut, self.program.defines_ptr);
            let _ = CString::from_raw(source_mut);

            if ffi::tvm_parse_labels((self as *mut Context).cast(), (*lexer).tokens.cast()) != 0 {
                return Err(InterpretingError::ParseLabelsError);
            }
            if ffi::tvm_parse_program((self as *mut Context).cast(), (*lexer).tokens.cast()) != 0 {
                return Err(InterpretingError::ParseProgramError);
            }

            ffi::tvm_lexer_destroy(lexer);
        }

        Ok(())
    }

    pub fn run(self: &mut Context) {
        let memory = unsafe { Pin::get_unchecked_mut(Pin::as_mut(&mut self.memory)) };
        memory.set_current_instruction_index(self.program.start_instruction_index as isize);

        loop {
            let current_instruction_index = self.memory.get_current_instruction_index();
            if current_instruction_index < 0
                || current_instruction_index > self.program.num_instructions as isize
            {
                panic!("Tried to read instruction outside the program area");
            }

            if unsafe {
                *self
                    .program
                    .instructions
                    .offset(self.memory.get_current_instruction_index() as isize)
            } == -1
            {
                break;
            }

            self.step();

            let memory = unsafe { Pin::get_unchecked_mut(Pin::as_mut(&mut self.memory)) };
            memory.set_current_instruction_index(memory.get_current_instruction_index() + 1);
        }
    }

    pub fn step(self: &mut Context) {
        let instruction_index = self.memory.get_current_instruction_index();

        if instruction_index < 0 || instruction_index >= self.program.num_instructions as isize {
            panic!("Tried to read instruction outside the program area");
        }

        unsafe {
            let args = *self
                .program
                .args
                .offset(self.memory.get_current_instruction_index());
            let memory = Pin::get_unchecked_mut(Pin::as_mut(&mut self.memory));

            let instruction = *self.program.instructions.offset(instruction_index);
            match FromPrimitive::from_i32(instruction).unwrap() {
                Nop => {}
                Int => { /* unimplemented */ }
                Mov => {
                    **args = **args.offset(1);
                }
                Push => {
                    memory.push_stack(**args);
                }
                Pop => {
                    **args = memory.pop_stack();
                }
                Pushf => {
                    memory.push_stack(memory.flags);
                }
                Popf => {
                    memory.flags = memory.pop_stack();
                }
                Inc => {
                    **args = **args + 1;
                }
                Dec => {
                    **args = **args - 1;
                }
                Add => {
                    **args = **args + **args.offset(1);
                }
                Sub => {
                    **args = **args - **args.offset(1);
                }
                Mul => {
                    **args = **args * **args.offset(1);
                }
                Div => {
                    **args = **args / **args.offset(1);
                }
                Mod => {
                    memory.remainder = **args % **args.offset(1);
                }
                Rem => {
                    **args = self.memory.remainder;
                }
                Not => {
                    **args = !**args;
                }
                Xor => {
                    **args = **args ^ **args.offset(1);
                }
                Or => {
                    **args = **args | **args.offset(1);
                }
                And => {
                    **args = **args & **args.offset(1);
                }
                Shl => {
                    **args = **args << **args.offset(1);
                }
                Shr => {
                    **args = **args >> **args.offset(1);
                }
                Cmp => {
                    memory.flags = if **args == **args.offset(1) { 1 } else { 0 }
                        | if **args > **args.offset(1) { 2 } else { 0 }
                }
                Jmp => memory.set_current_instruction_index((**args - 1) as isize),
                Call => {
                    memory.push_stack(instruction_index as i32);
                    memory.set_current_instruction_index((**args - 1) as isize);
                }
                Ret => {
                    let target = memory.pop_stack();
                    memory.set_current_instruction_index(target as isize);
                }
                Je => {
                    if memory.flags & 0x1 != 0 {
                        memory.set_current_instruction_index((**args - 1) as isize);
                    }
                }
                Jne => {
                    if memory.flags & 0x1 == 0 {
                        memory.set_current_instruction_index((**args - 1) as isize);
                    }
                }
                Jg => {
                    if memory.flags & 0x2 != 0 {
                        memory.set_current_instruction_index((**args - 1) as isize);
                    }
                }
                Jge => {
                    if memory.flags & 0x3 != 0 {
                        memory.set_current_instruction_index((**args - 1) as isize);
                    }
                }
                Jl => {
                    if memory.flags & 0x3 == 0 {
                        memory.set_current_instruction_index((**args - 1) as isize);
                    }
                }
                Jle => {
                    if memory.flags & 0x2 == 0 {
                        memory.set_current_instruction_index((**args - 1) as isize);
                    }
                }
                Prn => println!("{}", **args),
            };
        }
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
pub unsafe extern "C" fn tvm_vm_create() -> *mut ffi::tvm_ctx {
    let context = Box::new(Context::new());
    Box::into_raw(context) as *mut ffi::tvm_ctx
}

#[no_mangle]
pub unsafe extern "C" fn tvm_vm_interpret(vm: *mut ffi::tvm_ctx, filename: *const c_char) -> c_int {
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
pub unsafe extern "C" fn tvm_vm_run(vm: *mut ffi::tvm_ctx) {
    let vm = &mut *(vm as *mut Context);

    vm.run();
}

#[no_mangle]
pub unsafe extern "C" fn tvm_step(vm: *mut ffi::tvm_ctx, _instr_idx: *mut c_int) {
    let vm = &mut *(vm as *mut Context);

    vm.step();
}

#[no_mangle]
pub unsafe extern "C" fn tvm_vm_destroy(vm: *mut ffi::tvm_ctx) {
    if vm.is_null() {
        return;
    }

    let vm = Box::from_raw(vm as *mut Context);

    drop(vm);
}
