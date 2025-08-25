use crate::misc::output::raw_print::print_line_ending;
use crate::x86::gdt::GDT_CODE;
use crate::x86::halt;
use crate::{debug, fatal, sync::mutex::Mutex, trace, x86::gdt::SharedGdtrAndIdtr};
use core::ptr::read_unaligned;

pub mod interrupt_control {
    pub fn disable_interrupts() {
        unsafe {
            // On x86, we use the cli instruction to disable interrupts. It stands for (Cl)ear
            // (i)nterrupt flag
            core::arch::asm!("cli");
            // trace!("Disabled interrupts");
            // This might be a design flaw but shush. We can't use this since the log() function
            // locks a mutex which uses this to disable interrupts. So if we were to do it it would
            // enter log() and try to lock the mutex again and fucking deadlock
        }
    }

    pub fn enable_interrupts() {
        unsafe {
            // On x86, we use the sti instruction to enable interrupts. It stands for (S)e(t)
            // (i)nterrupt flag
            core::arch::asm!("sti");
            // trace!("Enabled interrupts");
            // This might be a design flaw but shush. We can't use this since the log() function
            // locks a mutex which uses this to enable interrupts. So if we were to do it it would
            // enter log() and try to lock the mutex again and fucking deadlock
        }
    }

    pub fn interrupts_enabled() -> bool {
        let eflags: u32;
        unsafe {
            // On x86, the interrupt enable flag is stored in the eflags register. However
            // we do not have direct access to read or write to the eflags register. Thankfully
            // we do have the 'pushfd' and 'popfd' instructions which push/pop the eflags register
            // onto the stack. So we start off by pushing the eflags register onto the stack for
            // later use using the 'pushfd' instruction we discussed earlier. Now we have to
            // store the eflags somewhere where we can access it later. That's where we use a
            // (G)eneral (P)urpose register which is commonly abbreviated as a GP register. As the
            // prefix of the (e)flags register name suggests, we should be using a GP register that
            // has a 'e' prefix. The 'e' means that it's 32-bit wide. Available 32-bit GP registers
            // on x86 include 'eax', 'ebx', 'ecx', 'edx', 'esi', 'edi'. As we are using inline
            // assembly for this, we'll let the compiler choose which register it wants to use.
            // So we define we require a GP register that's contents will be then mirrored into
            // the 'eflags' variable we declared above. So now we need to pop the value we pushed
            // onto the stack with 'pushfd' into the GP register the compiler chose. For that we
            // will use the 'pop' instruction. To pop a value from the stack onto a register the
            // compiler chose. As this is inline assembly we can do these fancy curly brackets and
            // write '{0}' in them to tell the compiler that we want the first register we asked for.
            // However we may and should hint the compiler that we require the register to be a 'e'
            // prefix register, and we do that with the '{0:e}' fancy syntax.
            core::arch::asm!(
                    "pushfd",
                    "pop {0:e}",
                    out(reg) eflags,
            );
        }

        // The 9th bit in the eflags register is the interrupt enable flag.
        (eflags & (1 << 9)) != 0
    }
}

static IDTR: Mutex<SharedGdtrAndIdtr> = Mutex::new(SharedGdtrAndIdtr { limit: 0, base: 0 });
static ISR_GATES: Mutex<[u64; 256]> = Mutex::new([0; 256]);

#[derive(Debug, Clone, Copy)]
pub struct InterruptGate {
    pub offset: u32,
    pub segment_selector: u16,
    pub gate_type: u8,
    pub dpl: u8,
    pub present: bool,
}

impl InterruptGate {
    pub fn to_u64(&self) -> u64 {
        let mut gate = 0;
        gate |= (self.offset & 0xFFFF) as u64;
        gate |= (self.segment_selector as u64) << 16;
        gate |= ((self.gate_type & 0b1111) as u64) << 40;
        gate |= ((self.dpl & 0b11) as u64) << 45;
        gate |= (if self.present { 1 } else { 0 }) << 47;
        gate |= ((self.offset >> 16) as u64) << 48;
        gate
    }
}

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

        fatal!(
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

        debug!(
            "EAX ={:#010X} EBX ={:#010X} ECX    ={:#010X} EDX={:#010X}",
            unsafe { read_unaligned(&raw const isr_frame.eax) },
            unsafe { read_unaligned(&raw const isr_frame.ebx) },
            unsafe { read_unaligned(&raw const isr_frame.ecx) },
            unsafe { read_unaligned(&raw const isr_frame.edx) }
        );

        debug!(
            "ESI ={:#010X} EDI ={:#010X} EBP    ={:#010X} ESP={:#010X}",
            unsafe { read_unaligned(&raw const isr_frame.esi) },
            unsafe { read_unaligned(&raw const isr_frame.edi) },
            unsafe { read_unaligned(&raw const isr_frame.ebp) },
            unsafe { read_unaligned(&raw const isr_frame.esp) },
        );

        debug!(
            "EIP ={:#010X} CS  ={:#010X} EFLAGS ={:#010X}",
            unsafe { read_unaligned(&raw const isr_frame.eip) },
            unsafe { read_unaligned(&raw const isr_frame.cs) },
            unsafe { read_unaligned(&raw const isr_frame.eflags) },
        );

        interrupt_control::disable_interrupts();
        loop {
            halt();
        }
    }
}

pub fn set_interrupt_gate(gate: InterruptGate, idx: u8) {
    ISR_GATES.lock()[idx as usize] = gate.to_u64();
    trace!("ISR {} handler set", idx);
}

pub fn load_idt() {
    unsafe { core::arch::asm!("lidt [{idt_reg:e}]", idt_reg = in(reg) &raw const *IDTR.lock()) }
    trace!("Loaded IDTR");
}

pub fn store_idt(loc: *const SharedGdtrAndIdtr) {
    unsafe { core::arch::asm!("sidt [{idt_reg:e}]", idt_reg = in(reg) loc) }
    trace!("Stored IDTR");
}

pub fn init() {
    {
        let gates_lock = ISR_GATES.lock();
        let mut idtr_lock = IDTR.lock();
        idtr_lock.base = (&raw const *gates_lock) as u32;
        idtr_lock.limit = (gates_lock.len() * core::mem::size_of_val(&gates_lock[0]) - 1) as u16;
    }
    trace!("Initialized IDTR");
    load_idt();
    let idtr = SharedGdtrAndIdtr { base: 0, limit: 0 };
    store_idt(&raw const idtr);

    assert_eq!(
        unsafe { read_unaligned(&raw const IDTR.lock().limit) },
        unsafe { read_unaligned(&raw const idtr.limit) }
    );

    assert_eq!(
        unsafe { read_unaligned(&raw const IDTR.lock().base) },
        unsafe { read_unaligned(&raw const idtr.base) }
    );

    trace!("Loaded IDTR matches stored IDTR");

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

    trace!("Masked PIC");

    interrupt_control::enable_interrupts();

    debug!("Initialized IDT");
}
