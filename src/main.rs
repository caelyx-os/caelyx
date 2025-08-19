#![no_std]
#![no_main]
#![feature(negative_impls)]

use crate::{
    drvs::vga::init as vga_init,
    drvs::vga::print as vga_print,
    x86::{halt, idt::interrupt_control::disable_interrupts},
};

pub mod drvs;
pub mod sync;
pub mod util;
pub mod x86;

#[unsafe(no_mangle)]
pub extern "C" fn caelyx_kmain() {
    vga_init();
    panic!("Kernel halted.");
}

#[panic_handler]
pub fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    vga_print(info.message().as_str().unwrap_or("Unknown panic message"));
    disable_interrupts();
    loop {
        halt();
    }
}
