use crate::instruction::Register;
use std::{os::raw::c_int, ptr::null_mut};

const NUM_REGISTERS: usize = 17;

#[derive(Copy, Clone)]
pub union RegisterValue {
    pub value: i32,
    pub pointer: *mut i32,
}

pub struct Memory {
    pub flags: c_int,
    pub remainder: c_int,
    pub mem_space: Vec<i32>,
    pub registers: [RegisterValue; NUM_REGISTERS],
}

impl Memory {
    pub fn new(size: usize) -> Memory {
        Memory {
            flags: 0,
            remainder: 0,
            mem_space: vec![0; size / std::mem::size_of::<i32>()],
            registers: [RegisterValue {
                pointer: null_mut(),
            }; NUM_REGISTERS],
        }
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

    pub fn get_current_instruction_index(self: &Memory) -> i32 {
        unsafe { self.registers[Register::Eip as usize].value }
    }

    pub fn set_current_instruction_index(self: &mut Memory, value: i32) {
        self.registers[Register::Eip as usize].value = value;
    }
}
