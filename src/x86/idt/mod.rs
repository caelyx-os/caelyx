use crate::{println, sync::mutex::Mutex, x86::gdt::SharedGdtrAndIdtr};

pub mod interrupt_control {
    pub fn disable_interrupts() {
        unsafe {
            // On x86, we use the cli instruction to disable interrupts. It stands for (Cl)ear
            // (i)nterrupt flag
            core::arch::asm!("cli");
        }
    }

    pub fn enable_interrupts() {
        unsafe {
            // On x86, we use the sti instruction to enable interrupts. It stands for (S)e(t)
            // (i)nterrupt flag
            core::arch::asm!("sti");
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

pub fn set_interrupt_gate(gate: InterruptGate, idx: u8) {
    ISR_GATES.lock()[idx as usize] = gate.to_u64();
}

pub fn load_idt() {
    unsafe { core::arch::asm!("lidt [{idt_reg:e}]", idt_reg = in(reg) &raw const *IDTR.lock()) }
}

pub fn store_idt(loc: *const SharedGdtrAndIdtr) {
    unsafe { core::arch::asm!("sidt [{idt_reg:e}]", idt_reg = in(reg) loc) }
}

pub fn init() {
    {
        let gates_lock = ISR_GATES.lock();
        let mut idtr_lock = IDTR.lock();
        idtr_lock.base = (&raw const *gates_lock) as u32;
        idtr_lock.limit = (gates_lock.len() * core::mem::size_of_val(&gates_lock[0]) - 1) as u16;
    }
    load_idt();
    let idtr = SharedGdtrAndIdtr { base: 0, limit: 0 };
    store_idt(&raw const idtr);

    assert_eq!(
        unsafe { core::ptr::read_unaligned(&raw const IDTR.lock().limit) },
        unsafe { core::ptr::read_unaligned(&raw const idtr.limit) }
    );

    assert_eq!(
        unsafe { core::ptr::read_unaligned(&raw const IDTR.lock().base) },
        unsafe { core::ptr::read_unaligned(&raw const idtr.base) }
    );

    println!("IDT init..OK");
}
