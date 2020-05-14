use std::{env, ffi::CString, process::exit};
use tinyvm::ffi;

fn main() {
    let filename = CString::new(env::args().nth(1).unwrap()).unwrap();
    let filename = filename.as_ptr() as *mut _;

    unsafe {
        let vm = ffi::tvm_vm_create();
        if vm.is_null() || ffi::tvm_vm_interpret(vm, filename) != 0 {
            exit(1);
        }
        ffi::tvm_vm_run(vm);
        ffi::tvm_vm_destroy(vm);
    }
}
