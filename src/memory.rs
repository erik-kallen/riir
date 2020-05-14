use crate::ffi::{size_t, tvm_mem, tvm_reg_u};
use std::{convert::TryInto, marker::PhantomPinned, os::raw::c_int, pin::Pin, ptr::null_mut};

const NUM_REGISTERS: usize = 17;

#[repr(C)]
#[derive(Copy, Clone)]
pub union Register {
    pub value: i32,
    pub pointer: *mut i32,
}

#[repr(C)]
pub struct Memory {
    pub flags: c_int,
    pub remainder: c_int,
    pub mem_space_ptr: *mut u8,
    pub mem_space_size: c_int,
    pub registers_ptr: *mut tvm_reg_u,
    pub mem_space: Vec<u8>,
    pub registers: [Register; NUM_REGISTERS],
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
            registers: [Register {
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
        self.registers[0x6].pointer = pointer;
        self.registers[0x7].pointer = pointer;
    }

    pub unsafe fn push_stack(self: &mut Memory, item: c_int) {
        let pointer = self.registers[0x6].pointer.offset(-1);
        self.registers[0x6].pointer = pointer;
        *pointer = item;
    }

    pub unsafe fn pop_stack(self: &mut Memory) -> c_int {
        let pointer = self.registers[0x6].pointer;
        self.registers[0x6].pointer = pointer.offset(1);
        *pointer
    }

    pub fn get_current_instruction_index(self: &Memory) -> isize {
        unsafe { self.registers[0x8].value as isize }
    }

    pub fn set_current_instruction_index(self: &mut Memory, value: isize) {
        self.registers[0x8].value = value as i32;
    }
}

#[no_mangle]
pub unsafe extern "C" fn tvm_mem_create(size: size_t) -> *mut tvm_mem {
    let mem = Memory::new(size.try_into().unwrap());
    let mem_inner = Pin::into_inner_unchecked(mem);
    Box::into_raw(mem_inner) as *mut tvm_mem
}

#[no_mangle]
pub unsafe extern "C" fn tvm_mem_destroy(mem: *mut tvm_mem) {
    if mem.is_null() {
        return;
    }
    let mem = Box::from_raw(mem as *mut Memory);
    drop(mem);
}

#[no_mangle]
pub unsafe extern "C" fn tvm_stack_create(mem: *mut tvm_mem, size: size_t) {
    let mem = &mut *(mem as *mut Memory);
    mem.create_stack(size as usize);
}

#[no_mangle]
pub unsafe extern "C" fn tvm_stack_push(mem: *mut tvm_mem, item: *mut ::std::os::raw::c_int) {
    let mem = &mut *(mem as *mut Memory);
    mem.push_stack(*item);
}

#[no_mangle]
pub unsafe extern "C" fn tvm_stack_pop(mem: *mut tvm_mem, dest: *mut ::std::os::raw::c_int) {
    let mem = &mut *(mem as *mut Memory);
    *dest = mem.pop_stack();
}
