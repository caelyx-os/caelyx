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
        (eflags & (1 << 9)) != 0
    }
}
