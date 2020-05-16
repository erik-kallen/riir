use crate::htab::{tvm_htab_create, HashTable};
use crate::instruction::Instruction;
use std::{marker::PhantomPinned, os::raw::c_void, pin::Pin, ptr::null_mut};

#[repr(C)]
pub struct Program {
    pub start_instruction_index: i32,
    pub instructions: Vec<Instruction>,
    pub defines_ptr: *mut c_void,
    pub label_htab_ptr: *mut c_void,
    pub labels: HashTable, // For some reason, the labels are owned and should be freed by the Program but the defines are not (they are freed by the lexer)
    pin: PhantomPinned,
}

impl Program {
    pub fn new() -> Pin<Box<Program>> {
        let mut program = Box::pin(Program {
            start_instruction_index: 0,
            instructions: vec![],
            defines_ptr: unsafe { tvm_htab_create() },
            label_htab_ptr: null_mut(),
            labels: HashTable::default(),
            pin: PhantomPinned,
        });
        unsafe {
            let program_mut = Pin::get_unchecked_mut(Pin::as_mut(&mut program));
            program_mut.label_htab_ptr = &mut program_mut.labels as *mut _ as *mut _;
        }

        program
    }
}
