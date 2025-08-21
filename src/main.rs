#![no_std]
#![no_main]
#![feature(negative_impls)]
#![feature(fn_traits)]

use crate::{
    boot::multiboot2,
    drvs::vga::init as vga_init,
    x86::{
        gdt::init as gdt_init,
        halt,
        idt::{init as idt_init, interrupt_control::disable_interrupts},
    },
};

pub mod boot;
pub mod drvs;
pub mod sync;
pub mod util;
pub mod x86;

#[unsafe(no_mangle)]
extern "C" fn caelyx_kmain(mb2_info: *const ()) {
    vga_init();
    gdt_init();
    idt_init();
    let iter = multiboot2::TagIterator::new(mb2_info);

    for tag in iter {
        println!("{tag:?}");
    }

    panic!("Finished all work");
}

#[panic_handler]
pub fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    if let Some(loc) = info.location() {
        println!("panic occured at {}:{}", loc.file(), loc.line());
    } else {
        println!("panic occured");
    }

    println!("\tmessage: \"{}\"", info.message());
    disable_interrupts();
    println!("kernel halted");
    loop {
        halt();
    }
}
