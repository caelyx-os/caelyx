#![no_std]
#![no_main]

#[repr(C, packed)]
pub struct Multiboot2InfoInner<T>
where
    T: Sized,
{
    pub magic: u32,
    pub architecture: u32,
    pub header_length: u32,
    pub checksum: u32,
    pub tags: T,
}

#[repr(C, packed)]
pub struct Multiboot2TagInner<T>
where
    T: Sized,
{
    pub type_: u16,
    pub flags: u16,
    pub size: u32,
    pub data: T,
}

#[repr(align(8))]
pub struct Multiboot2Info<T>(Multiboot2InfoInner<T>);

#[repr(align(8))]
pub struct Multiboot2Tag<T>(Multiboot2TagInner<T>);

#[unsafe(no_mangle)]
pub extern "C" fn caelyx_kmain() {}

#[panic_handler]
pub fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    unreachable!();
}

