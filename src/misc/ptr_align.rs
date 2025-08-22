pub fn align_ptr_up(ptr: *const u8, align: usize) -> *const u8 {
    assert!(align.is_power_of_two(), "Alignment must be a power of two");
    let addr = ptr as usize;
    let aligned = (addr + align - 1) & !(align - 1);
    aligned as *const u8
}
