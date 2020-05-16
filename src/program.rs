use crate::{
    htab::{HashTable, tvm_htab_create},
    parser::NUM_ARGS,
};
use std::{marker::PhantomPinned, os::raw::{c_int, c_void}, pin::Pin, ptr::null_mut};

#[repr(C)]
pub struct Program {
    pub start_instruction_index: c_int,
    pub num_instructions: c_int,
    pub instructions: *mut c_int,
    pub args: *mut *mut *mut c_int,
    pub values: *mut *mut c_int,
    pub num_values: c_int,
    pub defines_ptr: *mut c_void,
    pub label_htab_ptr: *mut c_void,
    pub labels: HashTable, // For some reason, the labels are owned and should be freed by the Program but the defines are not (they are freed by the lexer)
    pin: PhantomPinned,
}

impl Program {
    pub fn new() -> Pin<Box<Program>> {
        let mut program = Box::pin(Program {
            start_instruction_index: 0,
            num_instructions: 0,
            instructions: null_mut(),
            args: null_mut(),
            values: null_mut(),
            num_values: 0,
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

impl Drop for Program {
    fn drop(self: &mut Program) {
        unsafe {
            if !self.values.is_null() {
                for i in 0..self.num_values {
                    drop(Box::<c_int>::from_raw(*self.values.offset(i as isize) as *mut _));
                }
                drop(Vec::<*mut c_int>::from_raw_parts(self.values as *mut _, self.num_values as usize, self.num_values as usize));
            }

            if !self.args.is_null() {
                let mut i: isize = 0;
                loop {
                    let current_arg = *self.args.offset(i);
                    if current_arg.is_null() {
                        break;
                    }
                    drop(Vec::<*mut c_int>::from_raw_parts(current_arg, NUM_ARGS, NUM_ARGS));
                    i += 1;
                }
                drop(Vec::<*mut *mut c_int>::from_raw_parts(self.args, (self.num_instructions + 1) as usize, (self.num_instructions + 1) as usize));
            }

            if !self.instructions.is_null() {
                drop(Vec::<c_int>::from_raw_parts(self.instructions, (self.num_instructions + 1) as usize, (self.num_instructions + 1) as usize));
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn tvm_prog_create() -> *mut c_void {
    let program = Program::new();
    let program_inner = Pin::into_inner_unchecked(program);
    Box::into_raw(program_inner) as *mut _
}

#[no_mangle]
pub unsafe extern "C" fn tvm_prog_destroy(p: *mut c_void) {
    if p.is_null() {
        return;
    }
    let p = Box::from_raw(p as *mut Program);
    drop(p);
}
