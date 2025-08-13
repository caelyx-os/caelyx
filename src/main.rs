#![no_std]
#![no_main]

#[unsafe(no_mangle)]
pub extern "C" fn caelyx_kmain() {

}

#[panic_handler]
pub fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    unreachable!();
}