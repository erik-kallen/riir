use crate::{
    instruction::Register,
};
use std::{convert::TryInto, marker::PhantomPinned, os::raw::{c_int, c_void}, pin::Pin, ptr::null_mut};

const NUM_REGISTERS: usize = 17;

#[repr(C)]
#[derive(Copy, Clone)]
pub union RegisterValue {
    pub value: i32,
    pub pointer: *mut i32,
}

#[repr(C)]
pub struct Memory {
    pub flags: c_int,
    pub remainder: c_int,
    pub mem_space_ptr: *mut u8,
    pub mem_space_size: c_int,
    pub registers_ptr: *mut c_void,
    pub mem_space: Vec<u8>,
    pub registers: [RegisterValue; NUM_REGISTERS],
    _pin: PhantomPinned,
}

impl Memory {
    pub fn new(size: usize) -> Pin<Box<Memory>> {
        let mut mem = Box::pin(Memory {
            flags: 0,
            remainder: 0,
            mem_space_ptr: null_mut(),
            mem_space_size: size as c_int,
            registers_ptr: null_mut(),
            mem_space: vec![0; size],
            registers: [RegisterValue {
                pointer: null_mut(),
            }; NUM_REGISTERS],
            _pin: PhantomPinned,
        });
        unsafe {
            let mem_mut = Pin::get_unchecked_mut(Pin::as_mut(&mut mem));
            mem_mut.mem_space_ptr = mem_mut.mem_space.as_mut_ptr();
            mem_mut.registers_ptr = mem_mut.registers.as_mut_ptr() as *mut _;
        }

        mem
    }

    pub fn create_stack(self: &mut Memory, size: usize) {
        let pointer = unsafe { self.mem_space.as_mut_ptr().offset(size as isize) as *mut i32 };
        self.registers[Register::Esp as usize].pointer = pointer;
        self.registers[Register::Ebp as usize].pointer = pointer;
    }

    pub unsafe fn push_stack(self: &mut Memory, item: c_int) {
        let pointer = self.registers[Register::Esp as usize].pointer.offset(-1);
        self.registers[Register::Esp as usize].pointer = pointer;
        *pointer = item;
    }

    pub unsafe fn pop_stack(self: &mut Memory) -> c_int {
        let pointer = self.registers[Register::Esp as usize].pointer;
        self.registers[Register::Esp as usize].pointer = pointer.offset(1);
        *pointer
    }

    pub fn get_current_instruction_index(self: &Memory) -> isize {
        unsafe { self.registers[Register::Eip as usize].value as isize }
    }

    pub fn set_current_instruction_index(self: &mut Memory, value: isize) {
        self.registers[Register::Eip as usize].value = value as i32;
    }
}

#[no_mangle]
pub unsafe extern "C" fn tvm_mem_create(size: usize) -> *mut c_void {
    let mem = Memory::new(size.try_into().unwrap());
    let mem_inner = Pin::into_inner_unchecked(mem);
    Box::into_raw(mem_inner) as *mut _
}

#[no_mangle]
pub unsafe extern "C" fn tvm_mem_destroy(mem: *mut c_void) {
    if mem.is_null() {
        return;
    }
    let mem = Box::from_raw(mem as *mut Memory);
    drop(mem);
}

#[no_mangle]
pub unsafe extern "C" fn tvm_stack_create(mem: *mut c_void, size: usize) {
    let mem = &mut *(mem as *mut Memory);
    mem.create_stack(size as usize);
}

#[no_mangle]
pub unsafe extern "C" fn tvm_stack_push(mem: *mut c_void, item: *mut c_int) {
    let mem = &mut *(mem as *mut Memory);
    mem.push_stack(*item);
}

#[no_mangle]
pub unsafe extern "C" fn tvm_stack_pop(mem: *mut c_void, dest: *mut c_int) {
    let mem = &mut *(mem as *mut Memory);
    *dest = mem.pop_stack();
}
