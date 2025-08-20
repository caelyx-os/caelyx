#![no_std]
#![no_main]
#![feature(negative_impls)]
#![feature(fn_traits)]

use core::ptr::read_volatile;

use crate::{
    drvs::vga::init as vga_init,
    x86::{
        gdt::{GDT_CODE, init as gdt_init},
        halt,
        idt::{
            InterruptGate, init as idt_init,
            interrupt_control::{disable_interrupts, enable_interrupts},
            set_interrupt_gate,
        },
    },
};

pub mod drvs;
pub mod sync;
pub mod util;
pub mod x86;

unsafe extern "C" {
    static isr_stubs: [u32; 256];
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct ISRFrame {
    edi: u32,
    esi: u32,
    ebp: u32,
    esp: u32,
    ebx: u32,
    edx: u32,
    ecx: u32,
    eax: u32,
    int_no: u32,
    err_no: u32,
    eip: u32,
    cs: u32,
    eflags: u32,
}

#[unsafe(no_mangle)]
extern "C" fn isr_general_handler(frame: *const ISRFrame) {
    let isr_frame: &'static ISRFrame = unsafe { &*frame };
    if isr_frame.int_no < 32 {
        println!("");
        println!(r" -------------           -------------    ");
        println!(r"/             \          /             \  ");
        println!(r"|             |          |             |  ");
        println!(r"|             |          |             |  ");
        println!(r"|             |          |             |  ");
        println!(r"\             /          \             /  ");
        println!(r" -------------            -------------   ");
        println!(r"                                          ");
        println!(r"   -----------------------------------    ");
        println!(r"  /                                   \   ");
        println!(r" /                                     \  ");
        println!("");

        println!(
            "{}",
            match isr_frame.int_no {
                0 => "DIVISION ERROR",
                1 => "DEBUG",
                2 => "NON MASKABLE INTERRUPT",
                3 => "BREAKPOINT",
                4 => "OVERFLOW",
                5 => "BOUND RANGE EXCEEDED",
                6 => "INVALID OPCODE",
                7 => "DEVICE NOT AVAILABLE",
                8 => "DOUBLE FAULT",
                9 => "COPROCESSOR SEGMENT OVERRUN",
                10 => "INVALID TSS",
                11 => "SEGMENT NOT PRESENT",
                12 => "STACK SEGMENT FAULT",
                13 => "GENERAL PROTECTION FAULT",
                14 => "PAGE FAULT",
                16 => "FLOATING POINT EXCEPTION",
                17 => "ALIGNMENT CHECK",
                18 => "MACHINE CHECK",
                19 => "SIMD FLOATING POINT EXCEPTION",
                20 => "VIRTUALIZATION EXCEPTION",
                21 => "CONTROL PROTECTION EXCEPTION",
                28 => "HYPERVISOR INJECTION EXCEPTION",
                29 => "VMM COMMUNICATION EXCEPTION",
                30 => "SECURITY EXCEPTION",
                _ => "UNKNOWN EXCEPTION",
            }
        );

        println!(
            "EAX ={:#010X} EBX ={:#010X} ECX    ={:#010X} EDX={:#010X}",
            unsafe { read_volatile(&raw const isr_frame.eax) },
            unsafe { read_volatile(&raw const isr_frame.ebx) },
            unsafe { read_volatile(&raw const isr_frame.ecx) },
            unsafe { read_volatile(&raw const isr_frame.edx) }
        );

        println!(
            "ESI ={:#010X} EDI ={:#010X} EBP    ={:#010X} ESP={:#010X}",
            unsafe { read_volatile(&raw const isr_frame.esi) },
            unsafe { read_volatile(&raw const isr_frame.edi) },
            unsafe { read_volatile(&raw const isr_frame.ebp) },
            unsafe { read_volatile(&raw const isr_frame.esp) },
        );

        println!(
            "EIP ={:#010X} CS  ={:#010X} EFLAGS ={:#010X}",
            unsafe { read_volatile(&raw const isr_frame.eip) },
            unsafe { read_volatile(&raw const isr_frame.cs) },
            unsafe { read_volatile(&raw const isr_frame.eflags) },
        );

        disable_interrupts();
        loop {
            halt();
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn caelyx_kmain() {
    vga_init();
    gdt_init();
    idt_init();
    for (i, stub) in unsafe { isr_stubs }.iter().enumerate() {
        set_interrupt_gate(
            InterruptGate {
                segment_selector: GDT_CODE,
                gate_type: 0xE,
                dpl: 0,
                present: true,
                offset: *stub,
            },
            i.try_into().unwrap(),
        );
    }
    unsafe {
        core::arch::asm!("out dx, al", in("al") 0xFFu8, in("dx") 0x21);
        core::arch::asm!("out dx, al", in("al") 0xFFu8, in("dx") 0xA1);
    }
    enable_interrupts();
    unsafe {
        core::arch::asm!(
            "mov ah, 0",
            "mov bl, 0",
            "div bl",
            out("ah") _,
            out("bl") _,
        )
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
