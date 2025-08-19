pub mod idt;

// This halts the CPU (it can be woken up by a interrupt)
pub fn halt() {
    unsafe {
        core::arch::asm!("hlt");
    }
}
