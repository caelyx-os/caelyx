#![no_std]
#![no_main]
#![feature(negative_impls)]
#![feature(fn_traits)]
#![feature(str_from_raw_parts)]

extern crate alloc;

use crate::{
    boot::multiboot2,
    drvs::{e9::init as e9_init, serial::init as serial_init, vga::init as vga_init},
    misc::{
        acpi::init as acpi_init,
        output::{flanterm::init as flanterm_init, logger::init as logger_init},
    },
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
    let mut tag_iter = multiboot2::TagIterator::new(mb2_info);
    vga_init();
    serial_init();
    e9_init();
    flanterm_init(&mut tag_iter);
    tag_iter.reset_pos();
    logger_init();
    gdt_init();
    idt_init();
    pmm_init(&mut tag_iter);
    tag_iter.reset_pos();
    virt_page_alloc_init();
    vmm_init();
    heap_init();
    print_cpuid();
    acpi_init(&mut tag_iter);
    tag_iter.reset_pos();

    panic!("Finished all work");
}
