#![no_std]
#![no_main]
#![feature(negative_impls)]
#![feature(fn_traits)]

use crate::{
    boot::multiboot2,
    drvs::{e9::init as e9_init, serial::init as serial_init, vga::init as vga_init},
    misc::output::{logger::init as logger_init, raw_print::print_line_ending},
    mm::{pmm::init as pmm_init, vmm::init as vmm_init},
    x86::{
        cpuid::print_cpuid,
        gdt::init as gdt_init,
        halt,
        idt::{init as idt_init, interrupt_control::disable_interrupts},
    },
};

pub mod boot;
pub mod drvs;
pub mod misc;
pub mod mm;
pub mod sync;
pub mod x86;

#[unsafe(no_mangle)]
extern "C" fn caelyx_kmain(mb2_info: *const ()) -> ! {
    let _ = mb2_info;
    vga_init();
    serial_init();
    e9_init();
    logger_init();
    gdt_init();
    idt_init();
    let mut multiboot_iter = multiboot2::TagIterator::new(mb2_info);
    pmm_init(&mut multiboot_iter);
    vmm_init();
    print_cpuid();

    panic!("Finished all work");
}

#[panic_handler]
pub fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    print_line_ending();
    fatal!(r" -------------           -------------    ");
    fatal!(r"/             \          /             \  ");
    fatal!(r"|             |          |             |  ");
    fatal!(r"|             |          |             |  ");
    fatal!(r"|             |          |             |  ");
    fatal!(r"\             /          \             /  ");
    fatal!(r" -------------            -------------   ");
    fatal!(r"                                          ");
    fatal!(r"   -----------------------------------    ");
    fatal!(r"  /                                   \   ");
    fatal!(r" /                                     \  ");
    print_line_ending();

    if let Some(loc) = info.location() {
        fatal!("panic occured at {}:{}", loc.file(), loc.line());
    } else {
        fatal!("panic occured");
    }

    fatal!("\tmessage: \"{}\"", info.message());
    disable_interrupts();
    fatal!("kernel halted");
    loop {
        halt();
    }
}
