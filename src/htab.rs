use crate::ffi::tvm_htab_ctx;
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    os::raw::{c_char, c_int, c_void},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    value: c_int,
    opaque_value: Vec<u8>,
}

impl Item {
    pub(crate) fn integer(value: c_int) -> Item {
        Item {
            value,
            opaque_value: Vec::new(),
        }
    }

    pub(crate) fn opaque<V>(opaque_value: V) -> Item
    where
        V: Into<Vec<u8>>,
    {
        Item {
            value: 0,
            opaque_value: opaque_value.into(),
        }
    }

    pub(crate) unsafe fn from_void(pointer: *const c_void, length: c_int) -> Item {
        let opaque_value = if pointer.is_null() {
            Vec::<u8>::new()
        } else {
            std::slice::from_raw_parts(pointer.cast(), length as usize).to_owned()
        };

        Item::opaque(opaque_value)
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
#[repr(transparent)]
pub struct HashTable(pub(crate) HashMap<CString, Item>);

#[no_mangle]
pub unsafe extern "C" fn tvm_htab_create() -> *mut tvm_htab_ctx {
    let hashtable = Box::new(HashTable::default());
    Box::into_raw(hashtable).cast()
}

#[no_mangle]
pub unsafe extern "C" fn tvm_htab_destroy(htab: *mut tvm_htab_ctx) {
    if htab.is_null() {
        return;
    }

    let hashtable = Box::from_raw(htab as *mut HashTable);
    drop(hashtable);
}

#[no_mangle]
pub unsafe extern "C" fn tvm_htab_add(
    htab: *mut tvm_htab_ctx,
    key: *const c_char,
    value: c_int,
) -> c_int {
    let hashtable = &mut *(htab as *mut HashTable);
    let key = CStr::from_ptr(key).to_owned();

    hashtable.0.insert(key, Item::integer(value));

    0
}

#[no_mangle]
pub unsafe extern "C" fn tvm_htab_add_ref(
    htab: *mut tvm_htab_ctx,
    key: *const c_char,
    valptr: *const c_void,
    len: c_int,
) -> c_int {
    let hashtable = &mut *(htab as *mut HashTable);
    let key = CStr::from_ptr(key).to_owned();

    hashtable.0.insert(key, Item::from_void(valptr, len));

    0
}

#[no_mangle]
pub unsafe extern "C" fn tvm_htab_find(htab: *mut tvm_htab_ctx, key: *const c_char) -> c_int {
    let hashtable = &*(htab as *mut HashTable);
    let key = CStr::from_ptr(key).to_owned();

    match hashtable.0.get(&key) {
        Some(item) => item.value,
        None => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn tvm_htab_find_ref(
    htab: *mut tvm_htab_ctx,
    key: *const c_char,
) -> *const c_char {
    let hashtable = &*(htab as *mut HashTable);
    let key = CStr::from_ptr(key).to_owned();

    match hashtable.0.get(&key) {
        Some(item) => item.opaque_value.as_ptr().cast(),
        None => std::ptr::null(),
    }
}
