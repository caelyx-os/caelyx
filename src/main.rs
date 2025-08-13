#![no_std]
#![no_main]

#[unsafe(no_mangle)]
pub extern "C" fn caelyx_kmain() {
    panic!("Asd");
}

#[panic_handler]
pub fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        core::arch::asm!("cli");
        loop {
            core::arch::asm!("hlt");
        }
    }
}
