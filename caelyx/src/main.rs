#![no_std]
#![no_main]
#![feature(negative_impls)]
#![feature(fn_traits)]

extern crate alloc;

use crate::{
    boot::multiboot2,
    drvs::{e9::init as e9_init, serial::init as serial_init, vga::init as vga_init},
    misc::output::{flanterm::init as flanterm_init, logger::init as logger_init},
    mm::{
        heap::init as heap_init, pmm::init as pmm_init,
        virt_page_alloc::init as virt_page_alloc_init, vmm::init as vmm_init,
    },
    x86::{cpuid::print_cpuid, gdt::init as gdt_init, idt::init as idt_init},
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
    flanterm_init(&mut multiboot2::TagIterator::new(mb2_info));
    logger_init();
    gdt_init();
    idt_init();
    pmm_init(&mut multiboot2::TagIterator::new(mb2_info));
    virt_page_alloc_init();
    vmm_init();
    heap_init();
    print_cpuid();

    panic!("Finished all work");
}
