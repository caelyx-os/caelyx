#![no_std]
#![allow(clippy::missing_safety_doc)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

#[unsafe(no_mangle)]
pub extern "C" fn __ffsdi2(x: u64) -> i32 {
    if x == 0 {
        return 0;
    }
    x.trailing_zeros() as i32 + 1
}

include!(concat!(env!("OUT_DIR"), "/uacpi_bindings.rs"));
