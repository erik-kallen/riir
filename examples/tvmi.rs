use std::{env, ffi::CString, process::exit};
use tinyvm::context::{tvm_vm_create, tvm_vm_destroy, tvm_vm_interpret, tvm_vm_run};

fn main() {
    let filename = CString::new(env::args().nth(1).unwrap()).unwrap();
    let filename = filename.as_ptr() as *mut _;

    unsafe {
        let vm = tvm_vm_create();
        if vm.is_null() || tvm_vm_interpret(vm, filename) != 0 {
            exit(1);
        }
        tvm_vm_run(vm);
        tvm_vm_destroy(vm);
    }
}
