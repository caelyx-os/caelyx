// These are just some basic io port manipulation functions that use the in and out x86
// instructions to output/input some data from a io port
pub fn inb(port: u16) -> u8 {
    let rv: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") rv, in("dx") port);
    }
    rv
}

pub fn outb(port: u16, val: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("al") val, in("dx") port);
    }
}

pub fn inw(port: u16) -> u16 {
    let rv: u16;
    unsafe {
        core::arch::asm!("in ax, dx", out("ax") rv, in("dx") port);
    }
    rv
}

pub fn outw(port: u16, val: u16) {
    unsafe {
        core::arch::asm!("out dx, ax", in("ax") val, in("dx") port);
    }
}

pub fn inl(port: u16) -> u32 {
    let rv: u32;
    unsafe {
        core::arch::asm!("in eax, dx", out("eax") rv, in("dx") port);
    }
    rv
}

pub fn outl(port: u16, val: u32) {
    unsafe {
        core::arch::asm!("out dx, eax", in("eax") val, in("dx") port);
    }
}
